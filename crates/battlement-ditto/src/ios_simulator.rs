//! Ephemeral iOS Simulator selection, launch, diagnostics, and cleanup.

use std::{
  fs::{self, File},
  path::{Component, Path, PathBuf},
  process::{Command, ExitStatus, Stdio},
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
  config::model::Orientation as ProfileOrientation,
  player_supervision::SimulatorApp,
  wire::job::{Display, Orientation},
};

const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Host commands and limits used to own one Simulator device.
#[derive(Clone, Debug)]
pub struct SimulatorTools {
  pub xcrun: PathBuf,
  pub plutil: PathBuf,
  pub command_timeout: Duration,
  pub boot_timeout: Duration,
}

/// One selected and booted Ditto-owned Simulator.
pub struct IosSimulator {
  tools: SimulatorTools,
  udid: String,
  name: String,
  bundle_id: Option<String>,
  pid: Option<u32>,
  deleted: bool,
}

/// Installed device facts used to resolve the player job.
#[derive(Clone, Debug, PartialEq)]
pub struct SimulatorDisplay {
  pub display: Display,
  pub runtime: String,
  pub device_type: String,
}

#[derive(Deserialize)]
struct Catalog {
  runtimes: Vec<Runtime>,
  devicetypes: Vec<DeviceType>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Runtime {
  name: String,
  identifier: String,
  version: String,
  is_available: bool,
}

#[derive(Deserialize)]
struct DeviceType {
  name: String,
  identifier: String,
}

struct CommandResult {
  status: ExitStatus,
  stdout: Vec<u8>,
  stderr: Vec<u8>,
}

impl IosSimulator {
  /// Selects installed iOS components and boots one uniquely named device.
  pub fn create(
    tools: SimulatorTools,
    requested_device: &str,
    session_id: &str,
  ) -> Result<(Self, SimulatorDisplay)> {
    self::validate_tools(&tools)?;
    let catalog = self::catalog(&tools)?;
    let runtime = catalog
      .runtimes
      .iter()
      .filter(|runtime| runtime.is_available && runtime.identifier.contains("iOS"))
      .max_by(|left, right| {
        self::version_key(&left.version).cmp(&self::version_key(&right.version))
      })
      .with_context(|| self::runtime_error(&catalog.runtimes))?;
    let device = catalog
      .devicetypes
      .iter()
      .find(|device| device.name == requested_device)
      .with_context(|| self::device_error(requested_device, &catalog.devicetypes))?;
    let suffix = session_id
      .chars()
      .filter(|value| value.is_ascii_alphanumeric())
      .take(12)
      .collect::<String>();
    let name = format!("Battlement Ditto {suffix}");
    let created = self::simctl(
      &tools,
      &["create", &name, &device.identifier, &runtime.identifier],
      tools.command_timeout,
    )?;
    self::require_success(&created, "create Simulator device")?;
    let udid = String::from_utf8(created.stdout)?.trim().to_owned();
    ensure!(!udid.is_empty(), "simctl create omitted the device UDID");
    let mut simulator = Self {
      tools,
      udid,
      name,
      bundle_id: None,
      pid: None,
      deleted: false,
    };
    let facts: Result<SimulatorDisplay> = (|| {
      simulator.run_simctl(&["boot", simulator.udid.as_str()])?;
      simulator.wait_for_boot()?;
      let (width, height, scale) = simulator.screen_geometry()?;
      Ok(SimulatorDisplay {
        display: Display {
          width,
          height,
          scale,
          orientation: None,
          safe_area: [0, 0, width, height],
        },
        runtime: runtime.name.clone(),
        device_type: device.name.clone(),
      })
    })();
    match facts {
      Ok(facts) => Ok((simulator, facts)),
      Err(error) => {
        let cleanup = simulator.delete();
        Err(error.context(match cleanup {
          Ok(()) => "Simulator setup failed; the device was deleted".to_owned(),
          Err(cleanup) => format!("Simulator setup failed; cleanup also failed: {cleanup:#}"),
        }))
      }
    }
  }

  /// Returns the exact Ditto-owned device identifier.
  pub fn udid(&self) -> &str {
    &self.udid
  }

  /// Returns the unique Ditto-owned device name.
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Installs and launches an immutable application on explicit IPv4 loopback.
  pub fn install_and_launch(
    &mut self,
    app: &Path,
    session_url: &str,
    orientation: ProfileOrientation,
  ) -> Result<()> {
    ensure!(app.is_dir(), "iOS application bundle is missing");
    ensure!(
      session_url.starts_with("http://127.0.0.1:"),
      "Simulator session URL must use explicit IPv4 loopback"
    );
    let plist = app.join("Info.plist");
    let bundle = self::run(
      &self.tools.plutil,
      &[
        "-extract",
        "CFBundleIdentifier",
        "raw",
        "--",
        &plist.to_string_lossy(),
      ],
      self.tools.command_timeout,
    )?;
    self::require_success(&bundle, "read iOS bundle identifier")?;
    let bundle_id = String::from_utf8(bundle.stdout)?.trim().to_owned();
    ensure!(
      !bundle_id.is_empty(),
      "iOS application bundle identifier is empty"
    );
    self.run_simctl(&["install", self.udid.as_str(), &app.to_string_lossy()])?;
    let orientation = self::orientation_name(orientation);
    let launched = self.simctl_with_environment(
      &self.tools,
      &[
        "launch",
        "--terminate-running-process",
        self.udid.as_str(),
        &bundle_id,
        "--battlement-ditto-url",
        session_url,
        "--battlement-ditto-orientation",
        orientation,
      ],
      &[("SIMCTL_CHILD_BATTLEMENT_DITTO_URL", session_url)],
      self.tools.command_timeout,
    )?;
    self::require_success(&launched, "launch Simulator application")?;
    let output = String::from_utf8(launched.stdout)?;
    let pid = output
      .split_once(':')
      .and_then(|(_, value)| value.trim().parse::<u32>().ok())
      .context("simctl launch omitted the application PID")?;
    self.bundle_id = Some(bundle_id);
    self.pid = Some(pid);
    Ok(())
  }

  /// Retains logs scoped to the launched application process.
  pub fn retain_logs(&self, destination: &Path) -> Result<()> {
    let pid = self.pid.context("Simulator application was not launched")?;
    let predicate = format!("processIdentifier == {pid}");
    let output = self::simctl(
      &self.tools,
      &[
        "spawn",
        self.udid.as_str(),
        "log",
        "show",
        "--style",
        "compact",
        "--last",
        "1h",
        "--predicate",
        &predicate,
      ],
      self.tools.command_timeout,
    )?;
    self::require_success(&output, "collect Simulator application logs")?;
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    if let Some(parent) = destination.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::write(destination, bytes)?;
    Ok(())
  }

  /// Copies a completed app-relative recording into host-owned storage.
  pub fn copy_recording(&self, relative: &str, destination: &Path) -> Result<()> {
    let relative = Path::new(relative);
    ensure!(
      !relative.is_absolute(),
      "Simulator recording path must be app-relative"
    );
    ensure!(
      relative
        .components()
        .all(|component| matches!(component, Component::Normal(_))),
      "Simulator recording path escapes the application container"
    );
    let bundle = self
      .bundle_id
      .as_deref()
      .context("Simulator application was not launched")?;
    let container = self::simctl(
      &self.tools,
      &["get_app_container", self.udid.as_str(), bundle, "data"],
      self.tools.command_timeout,
    )?;
    self::require_success(&container, "resolve Simulator data container")?;
    let source = PathBuf::from(String::from_utf8(container.stdout)?.trim()).join(relative);
    ensure!(
      source.is_file(),
      "Simulator recording is unavailable: {}",
      source.display()
    );
    if let Some(parent) = destination.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
  }

  /// Shuts down and deletes the exact owned device.
  pub fn delete(&mut self) -> Result<()> {
    if self.deleted {
      return Ok(());
    }
    let shutdown = self::simctl(
      &self.tools,
      &["shutdown", self.udid.as_str()],
      self.tools.command_timeout,
    );
    let deleted = self::simctl(
      &self.tools,
      &["delete", self.udid.as_str()],
      self.tools.command_timeout,
    )?;
    self::require_success(&deleted, "delete Simulator device")?;
    self.deleted = true;
    if let Ok(output) = shutdown {
      if !output.status.success() && !String::from_utf8_lossy(&output.stderr).contains("Shutdown") {
        anyhow::bail!(
          "shutdown Simulator device: {}",
          String::from_utf8_lossy(&output.stderr).trim()
        );
      }
    }
    Ok(())
  }

  fn wait_for_boot(&self) -> Result<()> {
    let started = Instant::now();
    loop {
      let output = self::simctl(
        &self.tools,
        &["list", "devices", self.udid.as_str(), "--json"],
        self.tools.command_timeout,
      )?;
      if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("\"Booted\"") {
        return Ok(());
      }
      ensure!(
        started.elapsed() < self.tools.boot_timeout,
        "Simulator boot deadline expired"
      );
      thread::sleep(POLL_INTERVAL);
    }
  }

  fn screen_geometry(&self) -> Result<(u32, u32, f64)> {
    let width = self.getenv_u32("SIMULATOR_MAINSCREEN_WIDTH")?;
    let height = self.getenv_u32("SIMULATOR_MAINSCREEN_HEIGHT")?;
    let scale = self.getenv_f64("SIMULATOR_MAINSCREEN_SCALE")?;
    ensure!(
      width > 0 && height > 0 && scale > 0.0,
      "Simulator returned invalid display facts"
    );
    Ok((width, height, scale))
  }

  fn getenv_u32(&self, name: &str) -> Result<u32> {
    self
      .getenv(name)?
      .parse()
      .with_context(|| format!("parse Simulator {name}"))
  }

  fn getenv_f64(&self, name: &str) -> Result<f64> {
    self
      .getenv(name)?
      .parse()
      .with_context(|| format!("parse Simulator {name}"))
  }

  fn getenv(&self, name: &str) -> Result<String> {
    let output = self::simctl(
      &self.tools,
      &["getenv", self.udid.as_str(), name],
      self.tools.command_timeout,
    )?;
    self::require_success(&output, &format!("query Simulator {name}"))?;
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
  }

  fn run_simctl(&self, arguments: &[&str]) -> Result<()> {
    let output = self::simctl(&self.tools, arguments, self.tools.command_timeout)?;
    self::require_success(&output, &format!("simctl {}", arguments[0]))
  }

  fn simctl_with_environment(
    &self,
    tools: &SimulatorTools,
    arguments: &[&str],
    environment: &[(&str, &str)],
    timeout: Duration,
  ) -> Result<CommandResult> {
    let mut values = vec!["simctl"];
    values.extend_from_slice(arguments);
    self::run_with_environment(&tools.xcrun, &values, environment, timeout)
  }
}

impl SimulatorApp for IosSimulator {
  fn is_running(&mut self) -> Result<bool> {
    let Some(pid) = self.pid else {
      return Ok(false);
    };
    Ok(
      self::simctl(
        &self.tools,
        &[
          "spawn",
          self.udid.as_str(),
          "/bin/kill",
          "-0",
          &pid.to_string(),
        ],
        self.tools.command_timeout,
      )?
      .status
      .success(),
    )
  }

  fn terminate(&mut self) -> Result<()> {
    if let Some(bundle) = self.bundle_id.clone() {
      let _ = self::simctl(
        &self.tools,
        &["terminate", self.udid.as_str(), &bundle],
        self.tools.command_timeout,
      );
    }
    self.delete()
  }
}

impl Drop for IosSimulator {
  fn drop(&mut self) {
    let _ = self.delete();
  }
}

fn catalog(tools: &SimulatorTools) -> Result<Catalog> {
  let output = simctl(
    tools,
    &["list", "--json", "runtimes", "devicetypes"],
    tools.command_timeout,
  )?;
  require_success(&output, "list Simulator runtimes and device types")?;
  serde_json::from_slice(&output.stdout).context("decode simctl catalog")
}

fn validate_tools(tools: &SimulatorTools) -> Result<()> {
  ensure!(tools.xcrun.is_file(), "xcrun is unavailable");
  ensure!(tools.plutil.is_file(), "plutil is unavailable");
  ensure!(
    !tools.command_timeout.is_zero(),
    "Simulator command timeout is zero"
  );
  ensure!(
    !tools.boot_timeout.is_zero(),
    "Simulator boot timeout is zero"
  );
  Ok(())
}

fn simctl(tools: &SimulatorTools, arguments: &[&str], timeout: Duration) -> Result<CommandResult> {
  let mut values = vec!["simctl"];
  values.extend_from_slice(arguments);
  run(&tools.xcrun, &values, timeout)
}

fn run(program: &Path, arguments: &[&str], timeout: Duration) -> Result<CommandResult> {
  self::run_with_environment(program, arguments, &[], timeout)
}

fn run_with_environment(
  program: &Path,
  arguments: &[&str],
  environment: &[(&str, &str)],
  timeout: Duration,
) -> Result<CommandResult> {
  let token = Uuid::new_v4().simple().to_string();
  let stdout_path = std::env::temp_dir().join(format!("ditto-{token}.stdout"));
  let stderr_path = std::env::temp_dir().join(format!("ditto-{token}.stderr"));
  let stdout = File::create(&stdout_path)?;
  let stderr = File::create(&stderr_path)?;
  let mut child = Command::new(program)
    .args(arguments)
    .envs(environment.iter().copied())
    .stdin(Stdio::null())
    .stdout(Stdio::from(stdout))
    .stderr(Stdio::from(stderr))
    .spawn()
    .with_context(|| format!("launch {}", program.display()))?;
  let started = Instant::now();
  let status = loop {
    if let Some(status) = child.try_wait()? {
      break status;
    }
    if started.elapsed() >= timeout {
      let _ = child.kill();
      let _ = child.wait();
      let _ = fs::remove_file(&stdout_path);
      let _ = fs::remove_file(&stderr_path);
      anyhow::bail!("command deadline expired: {}", program.display());
    }
    thread::sleep(POLL_INTERVAL);
  };
  let result = CommandResult {
    status,
    stdout: fs::read(&stdout_path)?,
    stderr: fs::read(&stderr_path)?,
  };
  fs::remove_file(stdout_path)?;
  fs::remove_file(stderr_path)?;
  Ok(result)
}

fn require_success(output: &CommandResult, context: &str) -> Result<()> {
  ensure!(
    output.status.success(),
    "{context}: {}",
    String::from_utf8_lossy(&output.stderr).trim()
  );
  Ok(())
}

fn runtime_error(runtimes: &[Runtime]) -> String {
  let alternatives = runtimes
    .iter()
    .filter(|runtime| runtime.is_available)
    .map(|runtime| runtime.name.as_str())
    .collect::<Vec<_>>();
  format!(
    "no available iOS Simulator runtime; installed alternatives: {}",
    alternatives.join(", ")
  )
}

fn device_error(requested: &str, devices: &[DeviceType]) -> String {
  let alternatives = devices
    .iter()
    .map(|device| device.name.as_str())
    .collect::<Vec<_>>();
  format!(
    "Simulator device type {requested:?} is unavailable; installed alternatives: {}",
    alternatives.join(", ")
  )
}

fn version_key(version: &str) -> Vec<u32> {
  version
    .split('.')
    .map(|part| part.parse().unwrap_or(0))
    .collect()
}

fn orientation_name(orientation: ProfileOrientation) -> &'static str {
  match orientation {
    ProfileOrientation::Portrait => "portrait",
    ProfileOrientation::PortraitUpsideDown => "portrait-upside-down",
    ProfileOrientation::LandscapeLeft => "landscape-left",
    ProfileOrientation::LandscapeRight => "landscape-right",
  }
}

/// Applies the requested orientation to observed portrait display facts.
pub fn orient_display(
  mut facts: SimulatorDisplay,
  orientation: ProfileOrientation,
) -> SimulatorDisplay {
  facts.display.orientation = Some(match orientation {
    ProfileOrientation::Portrait => Orientation::Portrait,
    ProfileOrientation::PortraitUpsideDown => Orientation::PortraitUpsideDown,
    ProfileOrientation::LandscapeLeft => Orientation::LandscapeLeft,
    ProfileOrientation::LandscapeRight => Orientation::LandscapeRight,
  });
  if matches!(
    orientation,
    ProfileOrientation::LandscapeLeft | ProfileOrientation::LandscapeRight
  ) {
    std::mem::swap(&mut facts.display.width, &mut facts.display.height);
    facts.display.safe_area = [0, 0, facts.display.width, facts.display.height];
  }
  facts
}
