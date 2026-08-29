//! Fixed, immutable iOS Simulator player builds for Ditto.

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
    BUILD_LOG_FILE, BuildAccess, BuildCache, BuildFailure, BuildHandle, NearestBuildMismatch,
    PendingBuild, SOURCE_MANIFEST_FILE,
  },
  build_identity::{
    AppleToolchain, BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, NativeInput,
    RustToolchain,
  },
  fingerprint::{CaseSensitivity, FingerprintRequest, GeneratedInput, SourceManifest},
  macos_build_staging::ProjectStaging,
  unity_lease::UnityEditorLease,
};

const EDITOR_METHOD: &str = "Battlement.Editor.BattlementDittoBuild.BuildIosSimulator";
const PLAYER: &str = "BattlementDitto.app";
const RELEASE_DEBUG_CONFIG: &str = "profile.release.debug=\"line-tables-only\"";
const RELEASE_SPLIT_DEBUG_CONFIG: &str = "profile.release.split-debuginfo=\"off\"";

pub const STARTUP_IDENTITY_FILE: &str = "startup-identity.json";

/// Executables and versions that affect an iOS Simulator player build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IosBuildTools {
  pub unity_editor: PathBuf,
  pub unity_version: String,
  pub cargo: PathBuf,
  pub cargo_version: String,
  pub rustc_version: String,
  pub architecture: String,
  pub xcodebuild: PathBuf,
  pub xcode_version: String,
  pub sdk_version: String,
}

/// Validated inputs for the iOS Simulator player build pipeline.
#[derive(Clone, Debug)]
pub struct IosBuildRequest {
  pub repository: PathBuf,
  pub unity_project: PathBuf,
  pub rust_manifest: PathBuf,
  pub scene: PathBuf,
  pub suite: String,
  pub diagnostics: bool,
  pub generated_inputs: Vec<GeneratedInput>,
  pub native_inputs: Vec<NativeInput>,
  pub capture_adapter: CaptureAdapter,
  pub tools: IosBuildTools,
  pub resource_slots: PathBuf,
  pub cache: BuildCache,
}

/// Startup facts retained beside an immutable iOS Simulator player.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IosStartupIdentity {
  pub platform: String,
  pub capture_adapter: String,
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub unity_version: String,
  pub diagnostics: bool,
}

/// Whether a ready iOS Simulator player was newly created or exactly reused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IosBuildOutcome {
  Created,
  Reused,
}

/// A retained terminal iOS Simulator build failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IosBuildFailure {
  pub identity: BuildIdentity,
  pub phase: String,
  pub error_ids: Vec<String>,
  pub message: String,
  pub log_path: PathBuf,
}

/// Terminal result of selecting or building an immutable iOS Simulator player.
#[derive(Debug)]
pub enum IosBuildResult {
  Ready {
    build: BuildHandle,
    outcome: IosBuildOutcome,
  },
  Required {
    identity: BuildIdentity,
    nearest: Option<NearestBuildMismatch>,
  },
  Failed(IosBuildFailure),
}

/// Selects an exact iOS Simulator player and optionally permits a cache-miss build.
pub fn select_ios_player(request: &IosBuildRequest, allow_build: bool) -> Result<IosBuildResult> {
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
  match request.cache.acquire(
    &request.repository.canonicalize()?.to_string_lossy(),
    &request.suite,
    &identity,
    now,
  )? {
    BuildAccess::Reused(build) => {
      self::validate_startup_identity(&build, &self::startup_identity(request, &identity))?;
      self::validate_player(&build.player_path())?;
      Ok(IosBuildResult::Ready {
        build,
        outcome: IosBuildOutcome::Reused,
      })
    }
    BuildAccess::Build(pending) if allow_build => {
      self::build_pending(request, pending, source, now)
    }
    BuildAccess::Build(pending) => {
      let identity = pending.identity().clone();
      let nearest = request.cache.nearest_build_mismatch(
        &request.repository.canonicalize()?.to_string_lossy(),
        &request.suite,
        &identity,
        &source,
      )?;
      pending.discard()?;
      Ok(IosBuildResult::Required { identity, nearest })
    }
  }
}

/// Returns the fixed application bundle inside a ready build.
pub fn player_app(build: &BuildHandle) -> Result<PathBuf> {
  let player = build.player_path();
  self::validate_player(&player)?;
  Ok(player)
}

/// Reads and validates startup facts retained beside an iOS Simulator build.
pub fn ios_startup_identity(build: &BuildHandle) -> Result<IosStartupIdentity> {
  let actual: IosStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE))?)?;
  let identity = &build.metadata().identity;
  ensure!(
    actual.platform == "ios-simulator",
    "startup identity is not iOS Simulator"
  );
  ensure!(
    actual.capture_adapter == self::identity_input(identity, "capture-adapter.name")?,
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
    actual.unity_version == self::identity_input(identity, "unity")?,
    "startup Unity version does not match build metadata"
  );
  Ok(actual)
}

fn validate_request(request: &IosBuildRequest) -> Result<()> {
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
  ensure!(
    request.tools.unity_editor.is_file(),
    "Unity editor is not a file"
  );
  ensure!(
    request.tools.cargo.is_file(),
    "Cargo executable is not a file"
  );
  ensure!(
    request.tools.xcodebuild.is_file(),
    "xcodebuild is not a file"
  );
  for (name, value) in [
    ("Unity version", request.tools.unity_version.as_str()),
    ("Cargo version", request.tools.cargo_version.as_str()),
    ("rustc version", request.tools.rustc_version.as_str()),
    ("Xcode version", request.tools.xcode_version.as_str()),
    ("SDK version", request.tools.sdk_version.as_str()),
  ] {
    ensure!(!value.is_empty(), "{name} is empty");
  }
  self::rust_target(&request.tools.architecture)?;
  Ok(())
}

fn build_identity(request: &IosBuildRequest, source: &SourceManifest) -> Result<BuildIdentity> {
  BuildIdentity::derive(&BuildIdentityRequest {
    source_fingerprint: source.fingerprint.clone(),
    target: BuildTarget::IosSimulator,
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
    options: BTreeMap::from([
      ("editor-method".to_owned(), EDITOR_METHOD.to_owned()),
      ("profile".to_owned(), "release".to_owned()),
      ("xcode-sdk".to_owned(), "iphonesimulator".to_owned()),
    ]),
  })
}

fn build_pending(
  request: &IosBuildRequest,
  pending: PendingBuild,
  source: SourceManifest,
  now: u64,
) -> Result<IosBuildResult> {
  source.write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
  fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
  let target = self::rust_target(&request.tools.architecture)?;
  let target_directory = pending.path().join(".native");
  let mut cargo = Command::new(&request.tools.cargo);
  cargo
    .arg("rustc")
    .arg("--manifest-path")
    .arg(&request.rust_manifest)
    .args(["--target", target, "--target-dir"])
    .arg(&target_directory)
    .arg("--release")
    .arg("--lib")
    .args(["--crate-type", "staticlib"])
    .args(["--config", RELEASE_DEBUG_CONFIG])
    .args(["--config", RELEASE_SPLIT_DEBUG_CONFIG]);
  let cargo_output = self::run_logged(cargo, pending.path(), "rust")?;
  if !cargo_output.status.success() {
    return self::failed(pending, "rust", &cargo_output, now);
  }
  let plugin = target_directory
    .join(target)
    .join("release/libbattlement_rules.a");
  if !plugin.is_file() {
    return self::failed_message(
      pending,
      "rust",
      "Rust build omitted libbattlement_rules.a",
      now,
    );
  }

  let startup = self::startup_identity(request, pending.identity());
  let startup_bytes = self::json_bytes(&startup)?;
  fs::write(pending.path().join(STARTUP_IDENTITY_FILE), &startup_bytes)?;
  let _lease = UnityEditorLease::acquire(&request.resource_slots)?;
  let staging = ProjectStaging::ios(
    &request.unity_project,
    &plugin,
    &startup_bytes,
    &pending.path().join(".project-backup"),
  )?;
  let xcode_project = pending.path().join(".xcode");
  let unity_log = pending.path().join("unity.log");
  let mut unity = Command::new(&request.tools.unity_editor);
  unity
    .args(["-batchmode", "-nographics", "-quit", "-projectPath"])
    .arg(&request.unity_project)
    .args(["-buildTarget", "iOS", "-executeMethod", EDITOR_METHOD])
    .args(["-logFile"])
    .arg(&unity_log)
    .env("BATTLEMENT_DITTO_BUILD_PATH", &xcode_project)
    .env("BATTLEMENT_DITTO_SCENE_PATH", self::unity_scene(request)?)
    .env(
      "BATTLEMENT_DITTO_DIAGNOSTICS",
      if request.diagnostics { "1" } else { "0" },
    )
    .env(
      "BATTLEMENT_DITTO_IOS_SIMULATOR_ARCHITECTURE",
      self::xcode_architecture(&request.tools.architecture)?,
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

  let derived = pending.path().join(".derived");
  let mut xcodebuild = Command::new(&request.tools.xcodebuild);
  xcodebuild
    .args([
      "-project",
      "Unity-iPhone.xcodeproj",
      "-target",
      "Unity-iPhone",
    ])
    .args(["-configuration", "Release", "-sdk", "iphonesimulator"])
    .arg(format!(
      "SYMROOT={}",
      derived.join("Build/Products").display()
    ))
    .arg(format!(
      "OBJROOT={}",
      derived.join("Build/Intermediates.noindex").display()
    ))
    .arg(format!(
      "ARCHS={}",
      self::xcode_architecture(&request.tools.architecture)?
    ))
    .arg("ONLY_ACTIVE_ARCH=YES")
    .args(["CODE_SIGNING_ALLOWED=NO", "build"])
    .current_dir(&xcode_project);
  let xcode_output = self::run_logged(xcodebuild, pending.path(), "xcode")?;
  if !xcode_output.status.success() {
    return self::failed(pending, "xcode", &xcode_output, now);
  }
  let app = self::built_app(&derived)?;
  fs::rename(app, pending.path().join(PLAYER))?;
  self::validate_player(&pending.path().join(PLAYER))?;
  for path in [target_directory, xcode_project, derived] {
    fs::remove_dir_all(path)?;
  }
  if unity_log.exists() {
    fs::remove_file(unity_log)?;
  }
  Ok(IosBuildResult::Ready {
    build: pending.publish(Path::new(PLAYER), now)?.build,
    outcome: IosBuildOutcome::Created,
  })
}

fn built_app(derived: &Path) -> Result<PathBuf> {
  let products = derived.join("Build/Products/Release-iphonesimulator");
  let mut apps = fs::read_dir(&products)
    .with_context(|| format!("inspect Xcode products in {}", products.display()))?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.is_dir() && path.extension().is_some_and(|value| value == "app"))
    .collect::<Vec<_>>();
  apps.sort();
  ensure!(
    apps.len() == 1,
    "Xcode produced {} application bundles",
    apps.len()
  );
  Ok(apps.remove(0))
}

fn validate_player(player: &Path) -> Result<()> {
  ensure!(player.is_dir(), "iOS Simulator player bundle is missing");
  ensure!(
    player.join("Info.plist").is_file(),
    "iOS player omitted Info.plist"
  );
  Ok(())
}

fn failed(pending: PendingBuild, phase: &str, output: &Output, now: u64) -> Result<IosBuildResult> {
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
) -> Result<IosBuildResult> {
  self::append_log(pending.path(), format!("{message}\n").as_bytes())?;
  self::failed_with_ids(pending, phase, message.to_owned(), Vec::new(), now)
}

fn failed_with_ids(
  pending: PendingBuild,
  phase: &str,
  message: String,
  error_ids: Vec<String>,
  now: u64,
) -> Result<IosBuildResult> {
  let identity = pending.identity().clone();
  let retained = BuildFailure {
    phase: phase.to_owned(),
    error_ids: error_ids.clone(),
    message: message.clone(),
    failed_at_unix_s: now,
  };
  let failure_path = pending.fail(&retained)?;
  Ok(IosBuildResult::Failed(IosBuildFailure {
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

fn startup_identity(request: &IosBuildRequest, identity: &BuildIdentity) -> IosStartupIdentity {
  IosStartupIdentity {
    platform: "ios-simulator".to_owned(),
    capture_adapter: request.capture_adapter.name.clone(),
    build_fingerprint: identity.fingerprint.clone(),
    source_fingerprint: identity.source_fingerprint.clone(),
    unity_version: request.tools.unity_version.clone(),
    diagnostics: request.diagnostics,
  }
}

fn validate_startup_identity(build: &BuildHandle, expected: &IosStartupIdentity) -> Result<()> {
  ensure!(
    self::ios_startup_identity(build)? == *expected,
    "cached startup identity mismatch"
  );
  Ok(())
}

fn identity_input<'a>(identity: &'a BuildIdentity, name: &str) -> Result<&'a str> {
  identity
    .inputs
    .iter()
    .find(|input| input.name == name)
    .map(|input| input.value.as_str())
    .with_context(|| format!("build metadata omitted {name}"))
}

fn json_bytes(value: &IosStartupIdentity) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  Ok(bytes)
}

fn unity_scene(request: &IosBuildRequest) -> Result<String> {
  let scene = request.scene.strip_prefix(&request.unity_project)?;
  ensure!(!scene.as_os_str().is_empty(), "Unity scene path is empty");
  Ok(scene.to_string_lossy().replace('\\', "/"))
}

fn rust_target(architecture: &str) -> Result<&'static str> {
  match architecture {
    "aarch64" | "arm64" => Ok("aarch64-apple-ios-sim"),
    "x86_64" => Ok("x86_64-apple-ios"),
    _ => anyhow::bail!("unsupported iOS Simulator architecture: {architecture}"),
  }
}

fn xcode_architecture(architecture: &str) -> Result<&'static str> {
  match architecture {
    "aarch64" | "arm64" => Ok("arm64"),
    "x86_64" => Ok("x86_64"),
    _ => anyhow::bail!("unsupported Xcode Simulator architecture: {architecture}"),
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
