//! Fixed, immutable WebGL player builds for Ditto.

use std::{
  collections::BTreeMap,
  env, fs,
  fs::OpenOptions,
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
    BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, NativeInput, RustToolchain,
  },
  fingerprint::{CaseSensitivity, FingerprintRequest, GeneratedInput, SourceManifest},
  macos_build_staging::ProjectStaging,
  unity_lease::UnityEditorLease,
};

const EDITOR_METHOD: &str = "Battlement.Editor.BattlementDittoBuild.BuildWebgl";
const PLAYER: &str = "player";
const RUST_TARGET: &str = "wasm32-unknown-emscripten";
const RELEASE_DEBUG_CONFIG: &str = "profile.release.debug=\"line-tables-only\"";
const RELEASE_SPLIT_DEBUG_CONFIG: &str = "profile.release.split-debuginfo=\"off\"";

pub const STARTUP_IDENTITY_FILE: &str = "startup-identity.json";

/// Executables and versions that affect a WebGL player build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebglBuildTools {
  pub unity_editor: PathBuf,
  pub unity_version: String,
  pub cargo: PathBuf,
  pub cargo_version: String,
  pub rustc_version: String,
}

/// Validated inputs for the WebGL player build pipeline.
#[derive(Clone, Debug)]
pub struct WebglBuildRequest {
  pub repository: PathBuf,
  pub unity_project: PathBuf,
  pub rust_manifest: PathBuf,
  pub scene: PathBuf,
  pub suite: String,
  pub diagnostics: bool,
  pub generated_inputs: Vec<GeneratedInput>,
  pub native_inputs: Vec<NativeInput>,
  pub capture_adapter: CaptureAdapter,
  pub tools: WebglBuildTools,
  pub resource_slots: PathBuf,
  pub cache: BuildCache,
}

/// Startup facts retained beside an immutable WebGL player.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WebglStartupIdentity {
  pub platform: String,
  pub capture_adapter: String,
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub unity_version: String,
  pub diagnostics: bool,
}

/// Whether a ready WebGL player was newly created or exactly reused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WebglBuildOutcome {
  Created,
  Reused,
}

/// A retained terminal WebGL build failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebglBuildFailure {
  pub identity: BuildIdentity,
  pub phase: String,
  pub error_ids: Vec<String>,
  pub message: String,
  pub log_path: PathBuf,
}

/// Terminal result of selecting or building an immutable WebGL player.
#[derive(Debug)]
pub enum WebglBuildResult {
  Ready {
    build: BuildHandle,
    outcome: WebglBuildOutcome,
  },
  Required {
    identity: BuildIdentity,
    nearest: Option<NearestBuildMismatch>,
  },
  Failed(WebglBuildFailure),
}

/// Selects an exact WebGL player and optionally permits building a cache miss.
pub fn select_webgl_player(
  request: &WebglBuildRequest,
  allow_build: bool,
) -> Result<WebglBuildResult> {
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
      self::validate_player(build.player_path().as_path())?;
      Ok(WebglBuildResult::Ready {
        build,
        outcome: WebglBuildOutcome::Reused,
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
      Ok(WebglBuildResult::Required { identity, nearest })
    }
  }
}

/// Reads and validates startup facts retained beside a WebGL build.
pub fn webgl_startup_identity(build: &BuildHandle) -> Result<WebglStartupIdentity> {
  let actual: WebglStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE))?)?;
  let identity = &build.metadata().identity;
  ensure!(actual.platform == "webgl", "startup identity is not WebGL");
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

fn validate_request(request: &WebglBuildRequest) -> Result<()> {
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
  for (name, value) in [
    ("Unity version", request.tools.unity_version.as_str()),
    ("Cargo version", request.tools.cargo_version.as_str()),
    ("rustc version", request.tools.rustc_version.as_str()),
  ] {
    ensure!(!value.is_empty(), "{name} is empty");
  }
  self::emscripten(&request.tools.unity_editor)?;
  Ok(())
}

fn build_identity(request: &WebglBuildRequest, source: &SourceManifest) -> Result<BuildIdentity> {
  BuildIdentity::derive(&BuildIdentityRequest {
    source_fingerprint: source.fingerprint.clone(),
    target: BuildTarget::Webgl,
    unity_version: request.tools.unity_version.clone(),
    rust: RustToolchain {
      rustc_version: request.tools.rustc_version.clone(),
      cargo_version: request.tools.cargo_version.clone(),
      target: RUST_TARGET.to_owned(),
    },
    apple: None,
    diagnostics: request.diagnostics,
    capture_adapter: request.capture_adapter.clone(),
    native_inputs: request.native_inputs.clone(),
    options: BTreeMap::from([
      ("editor-method".to_owned(), EDITOR_METHOD.to_owned()),
      ("profile".to_owned(), "release".to_owned()),
      ("webgl-compression".to_owned(), "gzip".to_owned()),
    ]),
  })
}

fn build_pending(
  request: &WebglBuildRequest,
  pending: PendingBuild,
  source: SourceManifest,
  now: u64,
) -> Result<WebglBuildResult> {
  source.write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
  fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
  let target_directory = pending.path().join(".native");
  let emscripten = self::emscripten(&request.tools.unity_editor)?;
  let config_directory = target_directory.join("emscripten");
  fs::create_dir_all(&config_directory)?;
  let config = config_directory.join(".emscripten");
  fs::write(
    &config,
    format!(
      "LLVM_ROOT = {}\nBINARYEN_ROOT = {}\nNODE_JS = {}\n",
      self::python_string(&emscripten.join("llvm")),
      self::python_string(&emscripten.join("binaryen")),
      self::python_string(&emscripten.join("node/node")),
    ),
  )?;
  let mut paths = vec![
    emscripten.join("emscripten"),
    emscripten.join("llvm"),
    emscripten.join("binaryen/bin"),
    emscripten.join("node"),
  ];
  if let Some(existing) = env::var_os("PATH") {
    paths.extend(env::split_paths(&existing));
  }
  let mut cargo = Command::new(&request.tools.cargo);
  cargo
    .arg("rustc")
    .arg("--manifest-path")
    .arg(&request.rust_manifest)
    .args(["--target", RUST_TARGET, "--target-dir"])
    .arg(&target_directory)
    .arg("--release")
    .arg("--lib")
    .args(["--crate-type", "staticlib"])
    .args(["--config", RELEASE_DEBUG_CONFIG])
    .args(["--config", RELEASE_SPLIT_DEBUG_CONFIG])
    .env("EM_CONFIG", &config)
    .env("EM_CACHE", config_directory.join("cache"))
    .env(
      "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER",
      emscripten.join("emscripten/emcc"),
    )
    .env("PATH", env::join_paths(paths)?);
  let cargo_output = self::run_logged(cargo, pending.path(), "rust")?;
  if !cargo_output.status.success() {
    return self::failed(pending, "rust", &cargo_output, now);
  }
  let plugin = target_directory
    .join(RUST_TARGET)
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
  let staging = ProjectStaging::webgl(
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
    .args(["-buildTarget", "WebGL", "-executeMethod", EDITOR_METHOD])
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
  if let Err(error) = self::validate_player(&pending.path().join(PLAYER)) {
    return self::failed_message(pending, "unity", &error.to_string(), now);
  }
  fs::remove_dir_all(target_directory)?;
  if unity_log.exists() {
    fs::remove_file(unity_log)?;
  }
  Ok(WebglBuildResult::Ready {
    build: pending.publish(Path::new(PLAYER), now)?.build,
    outcome: WebglBuildOutcome::Created,
  })
}

fn validate_player(player: &Path) -> Result<()> {
  ensure!(
    player.join("index.html").is_file(),
    "WebGL player omitted index.html"
  );
  let build = player.join("Build");
  ensure!(build.is_dir(), "WebGL player omitted its Build directory");
  let names = fs::read_dir(build)?
    .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
    .collect::<Result<Vec<_>>>()?;
  ensure!(
    names.iter().any(|name| name.ends_with(".loader.js")),
    "WebGL player omitted its loader"
  );
  ensure!(
    names.iter().any(|name| name.contains(".wasm")),
    "WebGL player omitted its Wasm module"
  );
  Ok(())
}

fn emscripten(editor: &Path) -> Result<PathBuf> {
  let root = editor
    .ancestors()
    .nth(4)
    .context("Unity Editor path does not have the expected application layout")?
    .join("PlaybackEngines/WebGLSupport/BuildTools/Emscripten");
  for required in [
    root.join("emscripten/emcc"),
    root.join("llvm/clang"),
    root.join("binaryen/bin/wasm-opt"),
    root.join("node/node"),
  ] {
    ensure!(
      required.is_file(),
      "Unity Web Build Support is incomplete: {} was not found",
      required.display()
    );
  }
  Ok(root)
}

fn failed(
  pending: PendingBuild,
  phase: &str,
  output: &Output,
  now: u64,
) -> Result<WebglBuildResult> {
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
) -> Result<WebglBuildResult> {
  self::append_log(pending.path(), format!("{message}\n").as_bytes())?;
  self::failed_with_ids(pending, phase, message.to_owned(), Vec::new(), now)
}

fn failed_with_ids(
  pending: PendingBuild,
  phase: &str,
  message: String,
  error_ids: Vec<String>,
  now: u64,
) -> Result<WebglBuildResult> {
  let identity = pending.identity().clone();
  let retained = BuildFailure {
    phase: phase.to_owned(),
    error_ids: error_ids.clone(),
    message: message.clone(),
    failed_at_unix_s: now,
  };
  let failure_path = pending.fail(&retained)?;
  Ok(WebglBuildResult::Failed(WebglBuildFailure {
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

fn startup_identity(request: &WebglBuildRequest, identity: &BuildIdentity) -> WebglStartupIdentity {
  WebglStartupIdentity {
    platform: "webgl".to_owned(),
    capture_adapter: request.capture_adapter.name.clone(),
    build_fingerprint: identity.fingerprint.clone(),
    source_fingerprint: identity.source_fingerprint.clone(),
    unity_version: request.tools.unity_version.clone(),
    diagnostics: request.diagnostics,
  }
}

fn validate_startup_identity(build: &BuildHandle, expected: &WebglStartupIdentity) -> Result<()> {
  ensure!(
    self::webgl_startup_identity(build)? == *expected,
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

fn json_bytes(value: &WebglStartupIdentity) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  Ok(bytes)
}

fn unity_scene(request: &WebglBuildRequest) -> Result<String> {
  let scene = request.scene.strip_prefix(&request.unity_project)?;
  ensure!(!scene.as_os_str().is_empty(), "Unity scene path is empty");
  Ok(scene.to_string_lossy().replace('\\', "/"))
}

fn python_string(path: &Path) -> String {
  format!(
    "'{}'",
    path
      .to_string_lossy()
      .replace('\\', "\\\\")
      .replace('\'', "\\'")
  )
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
