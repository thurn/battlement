//! Installs exact signed device builds and launches reproducible native fixtures.

use std::{fs, path::PathBuf, process::Command};

use anyhow::{Context, Result, ensure};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{build_cache::BuildHandle, build_control::BuildControl, ios_build};

pub struct IosDeviceTools {
  pub xcrun: PathBuf,
  pub codesign: PathBuf,
  pub plutil: PathBuf,
}

pub struct IosDeviceRequest {
  pub device: String,
  pub evidence: PathBuf,
  pub launch: bool,
  pub fixture: Option<String>,
}

#[derive(Serialize)]
pub struct IosDeviceReceipt {
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub bundle_identifier: String,
  pub device: String,
  pub fixture: Option<String>,
  pub installation: Value,
  pub launch: Option<Value>,
}

/// Verifies and installs the retained signed build, optionally leaving its app running.
pub fn install(
  build: &BuildHandle,
  tools: &IosDeviceTools,
  request: &IosDeviceRequest,
  control: BuildControl<'_>,
) -> Result<IosDeviceReceipt> {
  ensure!(
    !request.device.trim().is_empty(),
    "supply an explicit iOS device"
  );
  ensure!(
    request.launch || request.fixture.is_none(),
    "a fixture requires launch"
  );
  if request.evidence.exists() {
    ensure!(
      fs::read_dir(&request.evidence)?.next().is_none(),
      "device evidence directory must be empty"
    );
  }
  let identity = ios_build::ios_startup_identity(build)?;
  ensure!(
    identity.platform == "ios-device",
    "a simulator build cannot be installed on a device"
  );
  let input = |key| {
    build
      .metadata()
      .identity
      .inputs
      .iter()
      .find(|entry| entry.name == key)
  };
  ensure!(
    input("option.signing").is_some_and(|entry| entry.value == "manual"),
    "an unsigned build cannot be installed on a device"
  );
  let bundle = input("option.bundle-identifier")
    .context("device build omitted its bundle identifier")?
    .value
    .clone();
  let app = ios_build::player_app(build)?;
  let profile = fs::read(app.join("embedded.mobileprovision"))
    .context("signed device app omitted its provisioning profile")?;
  let profile_hash: String = Sha256::digest(profile)
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect();
  ensure!(
    input("option.signing-profile-fingerprint").is_some_and(|entry| entry.value == profile_hash),
    "embedded provisioning profile does not match retained signing inputs"
  );
  let output = control.output(
    Command::new(&tools.plutil)
      .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
      .arg(app.join("Info.plist")),
  )?;
  ensure!(
    output.status.success(),
    "cannot read device bundle identifier"
  );
  ensure!(
    String::from_utf8(output.stdout)?.trim() == bundle,
    "device bundle identifier does not match retained build identity"
  );
  let signature = control.output(
    Command::new(&tools.codesign)
      .args(["--verify", "--deep", "--strict"])
      .arg(&app),
  )?;
  ensure!(
    signature.status.success(),
    "device signature is invalid: {}",
    String::from_utf8_lossy(&signature.stderr)
  );
  fs::create_dir_all(&request.evidence)?;
  let installation = self::invoke(
    tools,
    request,
    "install",
    &[
      "device",
      "install",
      "app",
      "--device",
      &request.device,
      app.to_str().context("app path is not UTF-8")?,
    ],
    control,
  )?;
  let launch = if request.launch {
    let environment = request
      .fixture
      .as_ref()
      .map_or_else(
        || serde_json::json!({}),
        |fixture| serde_json::json!({"BATTLEMENT_DITTO_SEMANTIC_FIXTURE": fixture}),
      )
      .to_string();
    Some(self::invoke(
      tools,
      request,
      "launch",
      &[
        "device",
        "process",
        "launch",
        "--device",
        &request.device,
        "--terminate-existing",
        "--environment-variables",
        &environment,
        &bundle,
      ],
      control,
    )?)
  } else {
    None
  };
  let receipt = IosDeviceReceipt {
    build_fingerprint: identity.build_fingerprint,
    source_fingerprint: identity.source_fingerprint,
    bundle_identifier: bundle,
    device: request.device.clone(),
    fixture: request.fixture.clone(),
    installation,
    launch,
  };
  fs::write(
    request.evidence.join("receipt.json"),
    serde_json::to_vec_pretty(&receipt)?,
  )?;
  Ok(receipt)
}

fn invoke(
  tools: &IosDeviceTools,
  request: &IosDeviceRequest,
  operation: &str,
  arguments: &[&str],
  control: BuildControl<'_>,
) -> Result<Value> {
  let result = request.evidence.join(format!("{operation}.json"));
  // A previous successful receipt must never mask a failed invocation.
  if result.exists() {
    fs::remove_file(&result)?;
  }
  let output = control.output(
    Command::new(&tools.xcrun)
      .arg("devicectl")
      .args(arguments)
      .args(["--timeout", "60", "--json-output"])
      .arg(&result)
      .arg("--log-output")
      .arg(request.evidence.join(format!("{operation}.log"))),
  )?;
  fs::write(
    request.evidence.join(format!("{operation}.stdout.log")),
    [&output.stdout[..], &output.stderr[..]].concat(),
  )?;
  ensure!(
    output.status.success(),
    "iOS {operation} failed; inspect {}",
    request.evidence.display()
  );
  let value: Value =
    serde_json::from_slice(&fs::read(&result).context("devicectl omitted its JSON receipt")?)?;
  ensure!(
    value["info"]["outcome"] == "success",
    "iOS {operation} did not report success; inspect {}",
    result.display()
  );
  Ok(value)
}
