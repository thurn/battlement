//! Fixed, immutable macOS player builds for Ditto.

use std::{
  collections::BTreeMap,
  fs::{self, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
  process::{Command, Output},
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::{
  build_cache::{
    BUILD_LOG_FILE, BuildAccess, BuildCache, BuildFailure, BuildHandle, PendingBuild,
    SOURCE_MANIFEST_FILE,
  },
  build_identity::{
    AppleToolchain, BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, NativeInput,
    RustToolchain,
  },
  fingerprint::{CaseSensitivity, FingerprintRequest, GeneratedInput, SourceManifest},
  macos_build_staging::ProjectStaging,
  unity_lease::UnityEditorLease,
};

const EDITOR_METHOD: &str = "Battlement.Editor.BattlementDittoBuild.BuildMacos";
const PLAYER: &str = "BattlementDitto.app";
const PLAYER_EXECUTABLE: &str = "Contents/MacOS/BattlementDitto";
const RELEASE_DEBUG_CONFIG: &str = "profile.release.debug=\"line-tables-only\"";
const RELEASE_SPLIT_DEBUG_CONFIG: &str = "profile.release.split-debuginfo=\"off\"";

pub const STARTUP_IDENTITY_FILE: &str = "startup-identity.json";

/// Executables and versions that affect a macOS player build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacosBuildTools {
  pub unity_editor: PathBuf,
  pub unity_version: String,
  pub cargo: PathBuf,
  pub cargo_version: String,
  pub rustc_version: String,
  pub architecture: String,
  pub xcode_version: String,
  pub sdk_version: String,
}

/// Validated inputs for the one supported macOS player build pipeline.
#[derive(Clone, Debug)]
pub struct MacosBuildRequest {
  pub repository: PathBuf,
  pub unity_project: PathBuf,
  pub rust_manifest: PathBuf,
  pub scene: PathBuf,
  pub suite: String,
  pub diagnostics: bool,
  pub generated_inputs: Vec<GeneratedInput>,
  pub native_inputs: Vec<NativeInput>,
  pub capture_adapter: CaptureAdapter,
  pub tools: MacosBuildTools,
  pub resource_slots: PathBuf,
  pub cache: BuildCache,
}

/// Build facts embedded in the player and retained beside it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MacosStartupIdentity {
  pub platform: String,
  pub capture_adapter: String,
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub unity_version: String,
  pub diagnostics: bool,
}

/// Whether a ready macOS player was newly created or exactly reused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosBuildOutcome {
  Created,
  Reused,
}

/// A retained terminal failure that must not launch a player.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacosBuildFailure {
  pub identity: BuildIdentity,
  pub phase: String,
  pub error_ids: Vec<String>,
  pub message: String,
  pub log_path: PathBuf,
}

/// Terminal result of selecting or building an immutable macOS player.
#[derive(Debug)]
pub enum MacosBuildResult {
  Ready {
    build: BuildHandle,
    outcome: MacosBuildOutcome,
  },
  Failed(MacosBuildFailure),
}

/// Builds or reuses the exact macOS player selected by current inputs.
pub fn build_macos_player(request: &MacosBuildRequest) -> Result<MacosBuildResult> {
  self::validate_request(request)?;
  let source = SourceManifest::build(&FingerprintRequest {
    repository: request.repository.clone(),
    unity_project: request.unity_project.clone(),
    rust_manifest: request.rust_manifest.clone(),
    generated_inputs: request.generated_inputs.clone(),
    case_sensitivity: CaseSensitivity::Insensitive,
  })?;
  let identity = self::build_identity(request, &source)?;
  let now = self::unix_time()?;
  match request.cache.acquire(&request.suite, &identity, now)? {
    BuildAccess::Reused(build) => {
      self::validate_startup_identity(&build, &self::startup_identity(request, &identity))?;
      Ok(MacosBuildResult::Ready {
        build,
        outcome: MacosBuildOutcome::Reused,
      })
    }
    BuildAccess::Build(pending) => self::build_pending(request, pending, source, now),
  }
}

/// Returns the fixed executable inside a ready Ditto macOS application.
pub fn player_executable(build: &BuildHandle) -> Result<PathBuf> {
  let executable = build.player_path().join(PLAYER_EXECUTABLE);
  ensure!(executable.is_file(), "macOS player omitted its executable");
  Ok(executable)
}

/// Reads and validates the startup facts retained beside an immutable player.
pub fn macos_startup_identity(build: &BuildHandle) -> Result<MacosStartupIdentity> {
  let actual: MacosStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE))?)?;
  let identity = &build.metadata().identity;
  ensure!(actual.platform == "macos", "startup identity is not macOS");
  ensure!(
    actual.capture_adapter == identity_input(identity, "capture-adapter.name")?,
    "startup capture adapter does not match build metadata"
  );
  ensure!(
    actual.build_fingerprint == identity.fingerprint,
    "startup fingerprint does not match build metadata"
  );
  ensure!(
    actual.source_fingerprint == identity.source_fingerprint,
    "startup source fingerprint does not match build metadata"
  );
  ensure!(
    actual.unity_version == identity_input(identity, "unity")?,
    "startup Unity version does not match build metadata"
  );
  ensure!(
    actual.diagnostics == (identity_input(identity, "diagnostics")? == "enabled"),
    "startup diagnostics do not match build metadata"
  );
  Ok(actual)
}

fn identity_input<'a>(identity: &'a BuildIdentity, name: &str) -> Result<&'a str> {
  identity
    .inputs
    .iter()
    .find(|input| input.name == name)
    .map(|input| input.value.as_str())
    .with_context(|| format!("build metadata omitted {name}"))
}

fn validate_request(request: &MacosBuildRequest) -> Result<()> {
  ensure!(!request.suite.is_empty(), "build suite is empty");
  ensure!(request.repository.is_dir(), "repository is not a directory");
  ensure!(
    request.unity_project.is_dir(),
    "Unity project is not a directory"
  );
  ensure!(
    request.rust_manifest.is_file(),
    "Rust manifest is not a file"
  );
  ensure!(request.scene.is_file(), "Unity scene is not a file");
  for (name, value) in [
    ("Unity version", request.tools.unity_version.as_str()),
    ("Cargo version", request.tools.cargo_version.as_str()),
    ("rustc version", request.tools.rustc_version.as_str()),
    ("Xcode version", request.tools.xcode_version.as_str()),
    ("SDK version", request.tools.sdk_version.as_str()),
  ] {
    ensure!(!value.is_empty(), "{name} is empty");
  }
  ensure!(
    request.tools.unity_editor.is_file(),
    "Unity editor is not a file"
  );
  ensure!(
    request.tools.cargo.is_file(),
    "Cargo executable is not a file"
  );
  self::rust_target(&request.tools.architecture)?;
  Ok(())
}

fn build_identity(request: &MacosBuildRequest, source: &SourceManifest) -> Result<BuildIdentity> {
  let options = BTreeMap::from([
    ("editor-method".to_owned(), EDITOR_METHOD.to_owned()),
    ("profile".to_owned(), "release".to_owned()),
  ]);
  BuildIdentity::derive(&BuildIdentityRequest {
    source_fingerprint: source.fingerprint.clone(),
    target: BuildTarget::Macos,
    unity_version: request.tools.unity_version.clone(),
    rust: RustToolchain {
      rustc_version: request.tools.rustc_version.clone(),
      cargo_version: request.tools.cargo_version.clone(),
      target: self::rust_target(&request.tools.architecture)?.to_owned(),
    },
    apple: Some(AppleToolchain {
      xcode_version: request.tools.xcode_version.clone(),
      sdk_version: request.tools.sdk_version.clone(),
    }),
    diagnostics: request.diagnostics,
    capture_adapter: request.capture_adapter.clone(),
    native_inputs: request.native_inputs.clone(),
    options,
  })
}

fn build_pending(
  request: &MacosBuildRequest,
  pending: PendingBuild,
  source: SourceManifest,
  now: u64,
) -> Result<MacosBuildResult> {
  source.write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
  fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
  let target = self::rust_target(&request.tools.architecture)?;
  let target_directory = pending.path().join(".native");
  let mut cargo = Command::new(&request.tools.cargo);
  cargo
    .arg("build")
    .arg("--manifest-path")
    .arg(&request.rust_manifest)
    .args(["--target", target, "--target-dir"])
    .arg(&target_directory)
    .arg("--release")
    .arg("--lib")
    .args(["--config", RELEASE_DEBUG_CONFIG])
    .args(["--config", RELEASE_SPLIT_DEBUG_CONFIG]);
  let cargo_output = self::run_logged(cargo, pending.path(), "rust")?;
  if !cargo_output.status.success() {
    return self::failed(pending, "rust", &cargo_output, now);
  }
  let plugin = target_directory
    .join(target)
    .join("release/libbattlement_rules.dylib");
  if !plugin.is_file() {
    return self::failed_message(
      pending,
      "rust",
      "Rust build omitted libbattlement_rules.dylib",
      now,
    );
  }

  let startup = self::startup_identity(request, pending.identity());
  let startup_bytes = self::json_bytes(&startup)?;
  fs::write(pending.path().join(STARTUP_IDENTITY_FILE), &startup_bytes)?;
  let _lease = UnityEditorLease::acquire(&request.resource_slots)?;
  let staging = ProjectStaging::new(
    &request.unity_project,
    &plugin,
    &startup_bytes,
    &pending.path().join(".project-backup"),
  )?;
  let unity_log = pending.path().join("unity.log");
  let mut unity = Command::new(&request.tools.unity_editor);
  unity
    .args(["-batchmode", "-nographics", "-quit", "-projectPath"])
    .arg(&request.unity_project)
    .args([
      "-buildTarget",
      "StandaloneOSX",
      "-executeMethod",
      EDITOR_METHOD,
    ])
    .args(["-logFile"])
    .arg(&unity_log)
    .env("BATTLEMENT_DITTO_BUILD_PATH", pending.path().join(PLAYER))
    .env("BATTLEMENT_DITTO_SCENE_PATH", self::unity_scene(request)?)
    .env(
      "BATTLEMENT_DITTO_DIAGNOSTICS",
      if request.diagnostics { "1" } else { "0" },
    );
  let unity_output = self::run_logged(unity, pending.path(), "unity")?;
  if unity_log.is_file() {
    self::append_log(pending.path(), &fs::read(&unity_log)?)?;
  }
  if let Err(error) = staging.restore() {
    return self::failed_message(
      pending,
      "restore",
      &format!("restore Unity project after build: {error:#}"),
      now,
    );
  }
  if !unity_output.status.success() {
    return self::failed(pending, "unity", &unity_output, now);
  }
  let player = pending.path().join(PLAYER);
  if !player.join(PLAYER_EXECUTABLE).is_file() {
    return self::failed_message(
      pending,
      "unity",
      "Unity build omitted the macOS player",
      now,
    );
  }
  fs::remove_dir_all(target_directory)?;
  if unity_log.exists() {
    fs::remove_file(unity_log)?;
  }
  Ok(MacosBuildResult::Ready {
    build: pending.publish(Path::new(PLAYER), now)?.build,
    outcome: MacosBuildOutcome::Created,
  })
}

fn failed(
  pending: PendingBuild,
  phase: &str,
  output: &Output,
  now: u64,
) -> Result<MacosBuildResult> {
  let text = fs::read_to_string(pending.path().join(BUILD_LOG_FILE))?;
  self::failed_with_ids(
    pending,
    phase,
    format!("{phase} build exited with {}", output.status),
    self::error_ids(&text),
    now,
  )
}

fn failed_message(
  pending: PendingBuild,
  phase: &str,
  message: &str,
  now: u64,
) -> Result<MacosBuildResult> {
  self::append_log(pending.path(), format!("{message}\n").as_bytes())?;
  self::failed_with_ids(pending, phase, message.to_owned(), Vec::new(), now)
}

fn failed_with_ids(
  pending: PendingBuild,
  phase: &str,
  message: String,
  error_ids: Vec<String>,
  now: u64,
) -> Result<MacosBuildResult> {
  let identity = pending.identity().clone();
  let retained = BuildFailure {
    phase: phase.to_owned(),
    error_ids: error_ids.clone(),
    message: message.clone(),
    failed_at_unix_s: now,
  };
  let failure_path = pending.fail(&retained)?;
  Ok(MacosBuildResult::Failed(MacosBuildFailure {
    identity,
    phase: phase.to_owned(),
    error_ids,
    message,
    log_path: failure_path.join(BUILD_LOG_FILE),
  }))
}

fn run_logged(mut command: Command, staging: &Path, phase: &str) -> Result<Output> {
  self::append_log(staging, format!("==> {phase}\n").as_bytes())?;
  let output = command
    .output()
    .with_context(|| format!("launch {phase} build"))?;
  self::append_log(staging, &output.stdout)?;
  self::append_log(staging, &output.stderr)?;
  Ok(output)
}

fn append_log(staging: &Path, bytes: &[u8]) -> Result<()> {
  OpenOptions::new()
    .append(true)
    .open(staging.join(BUILD_LOG_FILE))?
    .write_all(bytes)?;
  Ok(())
}

fn startup_identity(request: &MacosBuildRequest, identity: &BuildIdentity) -> MacosStartupIdentity {
  MacosStartupIdentity {
    platform: "macos".to_owned(),
    capture_adapter: request.capture_adapter.name.clone(),
    build_fingerprint: identity.fingerprint.clone(),
    source_fingerprint: identity.source_fingerprint.clone(),
    unity_version: request.tools.unity_version.clone(),
    diagnostics: request.diagnostics,
  }
}

fn validate_startup_identity(build: &BuildHandle, expected: &MacosStartupIdentity) -> Result<()> {
  let actual: MacosStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE))?)?;
  ensure!(actual == *expected, "cached startup identity mismatch");
  Ok(())
}

fn json_bytes(value: &MacosStartupIdentity) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  Ok(bytes)
}

fn unity_scene(request: &MacosBuildRequest) -> Result<String> {
  let scene = request.scene.strip_prefix(&request.unity_project)?;
  ensure!(!scene.as_os_str().is_empty(), "Unity scene path is empty");
  Ok(scene.to_string_lossy().replace('\\', "/"))
}

fn rust_target(architecture: &str) -> Result<&'static str> {
  match architecture {
    "aarch64" | "arm64" => Ok("aarch64-apple-darwin"),
    "x86_64" => Ok("x86_64-apple-darwin"),
    _ => anyhow::bail!("unsupported macOS architecture: {architecture}"),
  }
}

fn error_ids(output: &str) -> Vec<String> {
  let mut ids = output
    .split(|character: char| character.is_whitespace() || matches!(character, '[' | ']' | ':'))
    .filter(|word| {
      let rust = word.starts_with('E') && word.len() == 5;
      let csharp = word.starts_with("CS") && word.len() == 6;
      (rust || csharp)
        && word
          .chars()
          .skip(if rust { 1 } else { 2 })
          .all(|value| value.is_ascii_digit())
    })
    .map(str::to_owned)
    .collect::<Vec<_>>();
  ids.sort();
  ids.dedup();
  ids
}

fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
