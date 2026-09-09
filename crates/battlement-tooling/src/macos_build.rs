//! Independently cached macOS shell, content, rules, and assembled players.

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
use sha2::{Digest, Sha256};

use crate::{
  build_cache::{
    BUILD_LOG_FILE, BuildAccess, BuildCache, BuildFailure, BuildHandle, NearestBuildMismatch,
    PendingBuild, SOURCE_MANIFEST_FILE,
  },
  build_identity::{BuildIdentity, CaptureAdapter, NativeInput},
  fingerprint::{CaseSensitivity, FingerprintRootsRequest, GeneratedInput, SourceManifest},
  unity_lease::{CompilerCapacityLease, UnityEditorLease},
};

const SHELL_METHOD: &str = "Battlement.Editor.BattlementDittoBuild.BuildMacosShell";
const CONTENT_METHOD: &str = "Battlement.Editor.BattlementDittoBuild.BuildMacosContent";
const PLAYER: &str = "BattlementDitto.app";
const PLAYER_EXECUTABLE: &str = "Contents/MacOS/BattlementDitto";
const PLAYER_PLUGIN: &str = "Contents/PlugIns/libbattlement_rules.dylib";
const PLAYER_CONTENT: &str = "Contents/Resources/Data/StreamingAssets/aa";
const PLAYER_ASSEMBLY: &str = "Contents/Resources/Battlement/assembly.json";
const PLAYER_REACTANT_CATALOG: &str = "Contents/Resources/Battlement/reactant-assets.json";
const REACTANT_CATALOG: &str =
  "Assets/Generated/BattlementReactant/Resources/BattlementReactantAssetCatalog.json";
const RULES_ARTIFACT: &str = "libbattlement_rules.dylib";
const CONTENT_ARTIFACT: &str = "aa";
const RECIPE_VERSION: &str = "1";
const SHELL_ASSETS: &[&str] = &[
  "Assets/AddressableAssetsData",
  "Assets/AddressableAssetsData.meta",
  "Assets/DefaultVolumeProfile.asset",
  "Assets/DefaultVolumeProfile.asset.meta",
  "Assets/Fonts",
  "Assets/Fonts.meta",
  "Assets/Resources",
  "Assets/Resources.meta",
  "Assets/Shaders",
  "Assets/Shaders.meta",
  "Assets/Settings",
  "Assets/UniversalRenderPipelineGlobalSettings.asset",
  "Assets/UniversalRenderPipelineGlobalSettings.asset.meta",
];
const SHELL_SUPPLEMENTAL_ASSETS: &[(&str, &str)] = &[
  (
    "samples/chess/Assets/Settings",
    "Assets/Resources/BattlementShellPipeline",
  ),
  (
    "samples/chess/Assets/Shaders/LegalSquare.shader",
    "Assets/Resources/BattlementShellPipeline/LegalSquare.shader",
  ),
  (
    "samples/chess/Assets/Shaders/LegalSquare.shader.meta",
    "Assets/Resources/BattlementShellPipeline/LegalSquare.shader.meta",
  ),
  (
    "samples/ui/Assets/Resources/BattlementTextSettings.asset",
    "Assets/Resources/BattlementTextSettings.asset",
  ),
  (
    "samples/ui/Assets/Resources/BattlementTextSettings.asset.meta",
    "Assets/Resources/BattlementTextSettings.asset.meta",
  ),
  (
    "samples/ui/Assets/Original/Battlement Emoji.asset",
    "Assets/Original/Battlement Emoji.asset",
  ),
  (
    "samples/ui/Assets/Original/Battlement Emoji.asset.meta",
    "Assets/Original/Battlement Emoji.asset.meta",
  ),
  (
    "samples/ui/Assets/Original/Rocket Emoji.png",
    "Assets/Original/Rocket Emoji.png",
  ),
  (
    "samples/ui/Assets/Original/Rocket Emoji.png.meta",
    "Assets/Original/Rocket Emoji.png.meta",
  ),
];
const REQUIRED_PLUGIN_EXPORTS: &[&str] = &[
  "battlement_buffer_free",
  "battlement_connect",
  "battlement_ditto_determinism_capabilities",
  "battlement_ditto_determinism_contract",
  "battlement_engine_create",
  "battlement_engine_destroy",
  "battlement_logging_drain",
  "battlement_native_abi_digest",
  "battlement_poll",
  "battlement_submit",
  "battlement_submit_ui_event",
  "battlement_wire_contract_digest",
];
const RELEASE_DEBUG_CONFIG: &str = "profile.release.debug=\"line-tables-only\"";
const RELEASE_SPLIT_DEBUG_CONFIG: &str = "profile.release.split-debuginfo=\"off\"";

pub const STARTUP_IDENTITY_FILE: &str = "startup-identity.json";

/// Executables and versions that affect macOS build components.
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

/// Inputs for a universal shell, sample content, and one Rust rules library.
#[derive(Clone, Debug)]
pub struct MacosBuildRequest {
  pub repository: PathBuf,
  pub unity_project: PathBuf,
  pub rust_manifest: PathBuf,
  pub scene: PathBuf,
  pub suite: String,
  pub diagnostics: bool,
  pub release_rules: bool,
  pub generated_inputs: Vec<GeneratedInput>,
  pub native_inputs: Vec<NativeInput>,
  pub capture_adapter: CaptureAdapter,
  pub tools: MacosBuildTools,
  pub resource_slots: PathBuf,
  pub cache: BuildCache,
}

/// Build facts reported by a running assembled player.
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

/// Independently selected component identities installed in an assembled app.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MacosAssemblyIdentity {
  #[serde(flatten)]
  pub startup: MacosStartupIdentity,
  pub suite: String,
  pub shell_fingerprint: String,
  pub content_fingerprint: String,
  pub rules_fingerprint: String,
}

/// Whether the final assembled app was newly created or exactly reused.
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

/// Terminal result of selecting or building an immutable assembled player.
#[derive(Debug)]
pub enum MacosBuildResult {
  Ready {
    build: BuildHandle,
    outcome: MacosBuildOutcome,
  },
  Required {
    identity: BuildIdentity,
    nearest: Option<NearestBuildMismatch>,
  },
  Failed(MacosBuildFailure),
}

struct ComponentIdentities {
  shell_source: SourceManifest,
  shell: BuildIdentity,
  content_source: SourceManifest,
  content: BuildIdentity,
  rules_source: SourceManifest,
  rules: BuildIdentity,
  assembly_source: SourceManifest,
  assembly: BuildIdentity,
}

struct StagedShellProject(PathBuf);

impl StagedShellProject {
  fn path(&self) -> &Path {
    &self.0
  }
}

impl Drop for StagedShellProject {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.0);
  }
}

/// Builds or reuses the exact assembled macOS player selected by current inputs.
pub fn build_macos_player(request: &MacosBuildRequest) -> Result<MacosBuildResult> {
  select_macos_player(request, true)
}

/// Selects an exact assembled macOS player and optionally permits cache misses.
pub fn select_macos_player(
  request: &MacosBuildRequest,
  allow_build: bool,
) -> Result<MacosBuildResult> {
  validate_request(request)?;
  let identities = component_identities(request)?;
  let now = unix_time()?;
  let repository = request
    .repository
    .canonicalize()?
    .to_string_lossy()
    .into_owned();
  match request
    .cache
    .acquire(&repository, &request.suite, &identities.assembly, now)?
  {
    BuildAccess::Reused(build) => {
      event("assembly", &identities.assembly, "reused", "exact-hit");
      validate_startup_identity(&build, &startup_identity(request, &identities.assembly))?;
      validate_assembly_identity(&build, request, &identities)?;
      Ok(MacosBuildResult::Ready {
        build,
        outcome: MacosBuildOutcome::Reused,
      })
    }
    BuildAccess::Build(pending) if allow_build => {
      event("assembly", &identities.assembly, "created", "cache-miss");
      build_assembly(request, pending, identities, &repository, now)
    }
    BuildAccess::Build(pending) => {
      let identity = pending.identity().clone();
      let nearest = request.cache.nearest_build_mismatch(
        &repository,
        &request.suite,
        &identity,
        &identities.assembly_source,
      )?;
      pending.discard()?;
      Ok(MacosBuildResult::Required { identity, nearest })
    }
  }
}

/// Returns the fixed executable inside a ready assembled application.
pub fn player_executable(build: &BuildHandle) -> Result<PathBuf> {
  let executable = build.player_path().join(PLAYER_EXECUTABLE);
  ensure!(executable.is_file(), "macOS player omitted its executable");
  Ok(executable)
}

/// Reads and validates startup facts retained beside an assembled player.
pub fn macos_startup_identity(build: &BuildHandle) -> Result<MacosStartupIdentity> {
  let actual: MacosStartupIdentity =
    serde_json::from_slice(&fs::read(build.path().join(STARTUP_IDENTITY_FILE))?)?;
  let identity = &build.metadata().identity;
  ensure!(actual.platform == "macos", "startup identity is not macOS");
  ensure!(
    actual.build_fingerprint == identity.fingerprint,
    "startup fingerprint does not match build metadata"
  );
  ensure!(
    actual.source_fingerprint == identity.source_fingerprint,
    "startup source fingerprint does not match build metadata"
  );
  Ok(actual)
}

/// Reads the component identities installed in an assembled player.
pub fn macos_assembly_identity(build: &BuildHandle) -> Result<MacosAssemblyIdentity> {
  let identity: MacosAssemblyIdentity =
    serde_json::from_slice(&fs::read(build.player_path().join(PLAYER_ASSEMBLY))?)?;
  ensure!(
    identity.startup == macos_startup_identity(build)?,
    "assembled app startup identity does not match cache metadata"
  );
  Ok(identity)
}

fn component_identities(request: &MacosBuildRequest) -> Result<ComponentIdentities> {
  let shell_project = shell_project(request);
  let contracts = contract_inputs(request)?;
  let mut shell_roots = vec![
    shell_project.join("Packages"),
    shell_project.join("ProjectSettings"),
    shell_package(request),
    request.repository.join("contracts/native-abi.json"),
    request.repository.join("contracts/wire-contract.json"),
  ];
  shell_roots.extend(
    SHELL_ASSETS
      .iter()
      .map(|path| shell_project.join(path))
      .filter(|path| path.exists()),
  );
  shell_roots.extend(
    SHELL_SUPPLEMENTAL_ASSETS
      .iter()
      .map(|(source, _)| request.repository.join(source))
      .filter(|path| path.exists()),
  );
  let shell_source = SourceManifest::build_roots(&FingerprintRootsRequest {
    repository: request.repository.clone(),
    roots: shell_roots,
    generated_inputs: vec![recipe_input("shell")],
    case_sensitivity: CaseSensitivity::Insensitive,
  })?;
  let shell = BuildIdentity::component(
    &shell_source.fingerprint,
    "macos-shell",
    BTreeMap::from([
      ("apple.sdk".to_owned(), request.tools.sdk_version.clone()),
      (
        "apple.xcode".to_owned(),
        request.tools.xcode_version.clone(),
      ),
      (
        "capture-adapter.name".to_owned(),
        request.capture_adapter.name.clone(),
      ),
      (
        "capture-adapter.version".to_owned(),
        request.capture_adapter.version.clone(),
      ),
      (
        "diagnostics".to_owned(),
        if request.diagnostics {
          "enabled"
        } else {
          "disabled"
        }
        .to_owned(),
      ),
      ("editor-method".to_owned(), SHELL_METHOD.to_owned()),
      ("target".to_owned(), "aarch64-apple-darwin".to_owned()),
      ("unity".to_owned(), request.tools.unity_version.clone()),
    ]),
  )?;

  let mut content_generated = request.generated_inputs.clone();
  content_generated.push(generated("shell-fingerprint", &shell.fingerprint));
  content_generated.push(recipe_input("content"));
  let content_source = SourceManifest::build_unity(
    &request.repository,
    &request.unity_project,
    &content_generated,
    CaseSensitivity::Insensitive,
  )?;
  let content = BuildIdentity::component(
    &content_source.fingerprint,
    "macos-addressables",
    BTreeMap::from([
      ("editor-method".to_owned(), CONTENT_METHOD.to_owned()),
      ("shell".to_owned(), shell.fingerprint.clone()),
      ("target".to_owned(), "aarch64-apple-darwin".to_owned()),
      ("unity".to_owned(), request.tools.unity_version.clone()),
    ]),
  )?;

  let rules_generated = request
    .native_inputs
    .iter()
    .chain(contracts.iter())
    .map(|input| generated(&format!("native-contract-{}", input.name), &input.sha256))
    .chain([recipe_input("rules")])
    .collect::<Vec<_>>();
  let rules_source = SourceManifest::build_rust(
    &request.repository,
    &request.rust_manifest,
    &rules_generated,
    CaseSensitivity::Insensitive,
  )?;
  let rules = BuildIdentity::component(
    &rules_source.fingerprint,
    "macos-rules",
    BTreeMap::from([
      ("cargo".to_owned(), request.tools.cargo_version.clone()),
      (
        "profile".to_owned(),
        if request.release_rules {
          "release"
        } else {
          "debug"
        }
        .to_owned(),
      ),
      ("rustc".to_owned(), request.tools.rustc_version.clone()),
      ("apple.sdk".to_owned(), request.tools.sdk_version.clone()),
      (
        "apple.xcode".to_owned(),
        request.tools.xcode_version.clone(),
      ),
      (
        "target".to_owned(),
        rust_target(&request.tools.architecture)?.to_owned(),
      ),
    ]),
  )?;

  let assembly_source = SourceManifest::build_roots(&FingerprintRootsRequest {
    repository: request.repository.clone(),
    roots: Vec::new(),
    generated_inputs: vec![
      generated("shell", &shell.fingerprint),
      generated("content", &content.fingerprint),
      generated("rules", &rules.fingerprint),
      generated("suite", &request.suite),
      recipe_input("assembly"),
    ],
    case_sensitivity: CaseSensitivity::Insensitive,
  })?;
  let assembly = BuildIdentity::component(
    &assembly_source.fingerprint,
    "macos-assembly",
    BTreeMap::from([
      ("assembler".to_owned(), recipe_digest()),
      ("content".to_owned(), content.fingerprint.clone()),
      ("rules".to_owned(), rules.fingerprint.clone()),
      ("shell".to_owned(), shell.fingerprint.clone()),
    ]),
  )?;
  Ok(ComponentIdentities {
    shell_source,
    shell,
    content_source,
    content,
    rules_source,
    rules,
    assembly_source,
    assembly,
  })
}

fn build_assembly(
  request: &MacosBuildRequest,
  pending: PendingBuild,
  identities: ComponentIdentities,
  repository: &str,
  now: u64,
) -> Result<MacosBuildResult> {
  identities
    .assembly_source
    .write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
  fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
  let rules = match resolve_rules(request, &identities, repository, now)? {
    Ok(build) => build,
    Err(failure) => {
      pending.discard()?;
      return Ok(MacosBuildResult::Failed(failure));
    }
  };
  let shell = match resolve_shell(request, &identities, repository, now)? {
    Ok(build) => build,
    Err(failure) => {
      pending.discard()?;
      return Ok(MacosBuildResult::Failed(failure));
    }
  };
  let content = match resolve_content(request, &identities, repository, now)? {
    Ok(build) => build,
    Err(failure) => {
      pending.discard()?;
      return Ok(MacosBuildResult::Failed(failure));
    }
  };

  let player = pending.path().join(PLAYER);
  clone_tree(&shell.player_path(), &player)?;
  let plugin = player.join(PLAYER_PLUGIN);
  fs::create_dir_all(plugin.parent().expect("plugin has a parent"))?;
  fs::copy(rules.player_path(), &plugin)?;
  let content_destination = player.join(PLAYER_CONTENT);
  if content_destination.exists() {
    fs::remove_dir_all(&content_destination)?;
  }
  fs::create_dir_all(content_destination.parent().expect("content has a parent"))?;
  clone_tree(&content.player_path(), &content_destination)?;

  let startup = startup_identity(request, &identities.assembly);
  let assembly = MacosAssemblyIdentity {
    startup: startup.clone(),
    suite: request.suite.clone(),
    shell_fingerprint: identities.shell.fingerprint,
    content_fingerprint: identities.content.fingerprint,
    rules_fingerprint: identities.rules.fingerprint,
  };
  fs::write(
    pending.path().join(STARTUP_IDENTITY_FILE),
    json_bytes(&startup)?,
  )?;
  let assembly_path = player.join(PLAYER_ASSEMBLY);
  fs::create_dir_all(
    assembly_path
      .parent()
      .expect("assembly identity has a parent"),
  )?;
  fs::write(&assembly_path, json_bytes(&assembly)?)?;
  let reactant_catalog = request.unity_project.join(REACTANT_CATALOG);
  if reactant_catalog.is_file() {
    fs::copy(reactant_catalog, player.join(PLAYER_REACTANT_CATALOG))?;
  }
  sign_assembled_player(&plugin, &player, pending.path())?;
  Ok(MacosBuildResult::Ready {
    build: pending.publish(Path::new(PLAYER), now)?.build,
    outcome: MacosBuildOutcome::Created,
  })
}

fn resolve_shell(
  request: &MacosBuildRequest,
  identities: &ComponentIdentities,
  repository: &str,
  now: u64,
) -> Result<std::result::Result<BuildHandle, MacosBuildFailure>> {
  match request
    .cache
    .acquire(repository, "__macos-shell", &identities.shell, now)?
  {
    BuildAccess::Reused(build) => {
      event("shell", &identities.shell, "reused", "exact-hit");
      Ok(Ok(build))
    }
    BuildAccess::Build(pending) => {
      event("shell", &identities.shell, "created", "cache-miss");
      identities
        .shell_source
        .write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
      fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
      let unity_log = pending.path().join("unity.log");
      let project = stage_shell_project(request, pending.path())?;
      let mut unity = unity_editor_command(request, "shell-miss")?;
      unity
        .args(["-batchmode", "-nographics", "-quit", "-projectPath"])
        .arg(project.path())
        .args([
          "-buildTarget",
          "StandaloneOSX",
          "-executeMethod",
          SHELL_METHOD,
        ])
        .args(["-logFile"])
        .arg(&unity_log)
        .env("BATTLEMENT_DITTO_BUILD_PATH", pending.path().join(PLAYER))
        .env(
          "BATTLEMENT_DITTO_DIAGNOSTICS",
          if request.diagnostics { "1" } else { "0" },
        );
      let _lease = UnityEditorLease::acquire(&request.resource_slots)?;
      let output = run_logged(unity, pending.path(), "shell")?;
      drop(project);
      append_unity_log(pending.path(), &unity_log)?;
      if !output.status.success() {
        return Ok(Err(failed(pending, "shell", &output, now)?));
      }
      let player = pending.path().join(PLAYER);
      if !player.join(PLAYER_EXECUTABLE).is_file() {
        return Ok(Err(failed_message(
          pending,
          "shell",
          "Unity shell build omitted the macOS player",
          now,
        )?));
      }
      cleanup_file(&unity_log)?;
      Ok(Ok(pending.publish(Path::new(PLAYER), now)?.build))
    }
  }
}

fn resolve_content(
  request: &MacosBuildRequest,
  identities: &ComponentIdentities,
  repository: &str,
  now: u64,
) -> Result<std::result::Result<BuildHandle, MacosBuildFailure>> {
  let suite = format!("{}::content", request.suite);
  match request
    .cache
    .acquire(repository, &suite, &identities.content, now)?
  {
    BuildAccess::Reused(build) => {
      event("content", &identities.content, "reused", "exact-hit");
      Ok(Ok(build))
    }
    BuildAccess::Build(pending) => {
      event("content", &identities.content, "created", "cache-miss");
      identities
        .content_source
        .write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
      fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
      let unity_log = pending.path().join("unity.log");
      let mut unity = unity_command(request, &request.unity_project, "content-miss")?;
      unity
        .args(["-batchmode", "-nographics", "-quit", "-projectPath"])
        .arg(&request.unity_project)
        .args([
          "-buildTarget",
          "StandaloneOSX",
          "-executeMethod",
          CONTENT_METHOD,
        ])
        .args(["-logFile"])
        .arg(&unity_log)
        .env(
          "BATTLEMENT_DITTO_CONTENT_PATH",
          pending.path().join(CONTENT_ARTIFACT),
        )
        .env("BATTLEMENT_DITTO_SCENE_PATH", unity_scene(request)?);
      let _lease = UnityEditorLease::acquire(&request.resource_slots)?;
      let output = run_logged(unity, pending.path(), "content")?;
      append_unity_log(pending.path(), &unity_log)?;
      if !output.status.success() {
        return Ok(Err(failed(pending, "content", &output, now)?));
      }
      if !pending.path().join(CONTENT_ARTIFACT).is_dir() {
        return Ok(Err(failed_message(
          pending,
          "content",
          "Unity Addressables build omitted the content directory",
          now,
        )?));
      }
      cleanup_file(&unity_log)?;
      Ok(Ok(pending.publish(Path::new(CONTENT_ARTIFACT), now)?.build))
    }
  }
}

fn resolve_rules(
  request: &MacosBuildRequest,
  identities: &ComponentIdentities,
  repository: &str,
  now: u64,
) -> Result<std::result::Result<BuildHandle, MacosBuildFailure>> {
  let suite = format!("{}::rules", request.suite);
  match request
    .cache
    .acquire(repository, &suite, &identities.rules, now)?
  {
    BuildAccess::Reused(build) => {
      event("rules", &identities.rules, "reused", "exact-hit");
      Ok(Ok(build))
    }
    BuildAccess::Build(pending) => {
      event("rules", &identities.rules, "created", "cache-miss");
      identities
        .rules_source
        .write(&pending.path().join(SOURCE_MANIFEST_FILE))?;
      fs::write(pending.path().join(BUILD_LOG_FILE), [])?;
      let target = rust_target(&request.tools.architecture)?;
      let target_directory = pending.path().join(".native");
      let mut cargo = Command::new(&request.tools.cargo);
      cargo
        .arg("build")
        .arg("--manifest-path")
        .arg(&request.rust_manifest)
        .args(["--target", target, "--target-dir"])
        .arg(&target_directory)
        .arg("--lib");
      if request.release_rules {
        cargo
          .arg("--release")
          .args(["--config", RELEASE_DEBUG_CONFIG])
          .args(["--config", RELEASE_SPLIT_DEBUG_CONFIG]);
      }
      let capacity = CompilerCapacityLease::acquire(&request.resource_slots)?;
      let output = run_logged(cargo, pending.path(), "rules")?;
      drop(capacity);
      if !output.status.success() {
        return Ok(Err(failed(pending, "rules", &output, now)?));
      }
      let plugin = target_directory
        .join(target)
        .join(if request.release_rules {
          "release"
        } else {
          "debug"
        })
        .join(RULES_ARTIFACT);
      if !plugin.is_file() {
        return Ok(Err(failed_message(
          pending,
          "rules",
          "Rust build omitted libbattlement_rules.dylib",
          now,
        )?));
      }
      fs::copy(plugin, pending.path().join(RULES_ARTIFACT))?;
      fs::remove_dir_all(target_directory)?;
      Ok(Ok(pending.publish(Path::new(RULES_ARTIFACT), now)?.build))
    }
  }
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
  ensure!(
    request.tools.cargo.is_file(),
    "Cargo executable is not a file"
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
  rust_target(&request.tools.architecture)?;
  Ok(())
}

fn shell_project(request: &MacosBuildRequest) -> PathBuf {
  let canonical = request.repository.join("samples/basic");
  if canonical.join("Packages").is_dir() && canonical.join("ProjectSettings").is_dir() {
    return canonical;
  }
  request.unity_project.clone()
}

fn shell_package(request: &MacosBuildRequest) -> PathBuf {
  let canonical = request.repository.join("Packages/com.battlement.client");
  if canonical.is_dir() {
    canonical
  } else {
    request.repository.join("package")
  }
}

fn stage_shell_project(request: &MacosBuildRequest, staging: &Path) -> Result<StagedShellProject> {
  let source = shell_project(request);
  let project = staging.join("shell-project");
  fs::create_dir_all(project.join("Assets"))?;
  clone_tree(&source.join("Packages"), &project.join("Packages"))?;
  let embedded_package = project.join("Packages/com.battlement.client");
  if !embedded_package.is_dir() {
    clone_tree(&shell_package(request), &embedded_package)?;
  }
  let manifest_path = project.join("Packages/manifest.json");
  let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
  let manifest = manifest
    .as_object_mut()
    .context("shell package manifest is not an object")?;
  manifest.remove("testables");
  manifest
    .get_mut("dependencies")
    .and_then(serde_json::Value::as_object_mut)
    .context("shell package dependencies are not an object")?
    .insert(
      "com.battlement.client".to_owned(),
      serde_json::Value::String("file:com.battlement.client".to_owned()),
    );
  fs::write(&manifest_path, json_bytes(&manifest)?)?;
  let lock_path = project.join("Packages/packages-lock.json");
  if lock_path.is_file() {
    let mut lock: serde_json::Value = serde_json::from_slice(&fs::read(&lock_path)?)?;
    if let Some(package) = lock["dependencies"]["com.battlement.client"].as_object_mut() {
      package.insert(
        "version".to_owned(),
        serde_json::Value::String("file:com.battlement.client".to_owned()),
      );
    }
    fs::write(lock_path, json_bytes(&lock)?)?;
  }
  clone_tree(
    &source.join("ProjectSettings"),
    &project.join("ProjectSettings"),
  )?;
  for relative in SHELL_ASSETS {
    let asset = source.join(relative);
    if !asset.exists() {
      continue;
    }
    let destination = project.join(relative);
    if asset.is_dir() {
      clone_tree(&asset, &destination)?;
    } else {
      fs::create_dir_all(destination.parent().expect("shell asset has a parent"))?;
      fs::copy(asset, destination)?;
    }
  }
  for (source, relative) in SHELL_SUPPLEMENTAL_ASSETS {
    let asset = request.repository.join(source);
    if !asset.exists() {
      continue;
    }
    let destination = project.join(relative);
    if asset.is_dir() {
      if destination.exists() {
        fs::remove_dir_all(&destination)?;
      }
      clone_tree(&asset, &destination)?;
    } else {
      fs::create_dir_all(destination.parent().expect("shell asset has a parent"))?;
      fs::copy(asset, destination)?;
    }
  }
  Ok(StagedShellProject(project))
}

fn contract_inputs(request: &MacosBuildRequest) -> Result<Vec<NativeInput>> {
  ["native-abi", "wire-contract"]
    .into_iter()
    .map(|name| {
      let path = request
        .repository
        .join("contracts")
        .join(format!("{name}.json"));
      let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
      Ok(NativeInput {
        name: name.to_owned(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
      })
    })
    .collect()
}

fn unity_command(request: &MacosBuildRequest, project: &Path, reason: &str) -> Result<Command> {
  unity_editor_command(request, reason)?;
  crate::transactional_unity_command(project, &request.tools.unity_editor)
}

fn unity_editor_command(request: &MacosBuildRequest, reason: &str) -> Result<Command> {
  ensure!(
    std::env::var_os("BATTLEMENT_UNITY_POLICY").as_deref() != Some("forbid".as_ref()),
    "Unity Editor launch forbidden by BATTLEMENT_UNITY_POLICY while resolving {reason}"
  );
  ensure!(
    request.tools.unity_editor.is_file(),
    "Unity Editor is required for {reason} but is unavailable at {}",
    request.tools.unity_editor.display()
  );
  eprintln!("process.start kind=unity-editor reason={reason}");
  Ok(Command::new(&request.tools.unity_editor))
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

fn validate_assembly_identity(
  build: &BuildHandle,
  request: &MacosBuildRequest,
  identities: &ComponentIdentities,
) -> Result<()> {
  let expected = MacosAssemblyIdentity {
    startup: startup_identity(request, &identities.assembly),
    suite: request.suite.clone(),
    shell_fingerprint: identities.shell.fingerprint.clone(),
    content_fingerprint: identities.content.fingerprint.clone(),
    rules_fingerprint: identities.rules.fingerprint.clone(),
  };
  ensure!(
    macos_assembly_identity(build)? == expected,
    "cached assembly identity mismatch"
  );
  Ok(())
}

fn sign_assembled_player(plugin: &Path, player: &Path, staging: &Path) -> Result<()> {
  if !player.join("Contents/Info.plist").is_file() {
    return Ok(());
  }
  verify_plugin(plugin, staging)?;
  for target in [plugin, player] {
    let mut command = Command::new("/usr/bin/codesign");
    command.args(["--force", "--sign", "-"]).arg(target);
    let output = run_logged(command, staging, "codesign")?;
    ensure!(
      output.status.success(),
      "codesign failed for {}",
      target.display()
    );
  }
  Ok(())
}

fn verify_plugin(plugin: &Path, staging: &Path) -> Result<()> {
  let mut lipo = Command::new("/usr/bin/lipo");
  lipo.args(["-archs"]).arg(plugin);
  let architectures = run_logged(lipo, staging, "verify-plugin-architecture")?;
  ensure!(
    architectures.status.success(),
    "lipo could not inspect rules plugin"
  );
  ensure!(
    String::from_utf8_lossy(&architectures.stdout)
      .split_whitespace()
      .any(|architecture| architecture == "arm64"),
    "rules plugin does not contain arm64"
  );
  let mut nm = Command::new("/usr/bin/nm");
  nm.args(["-gjU"]).arg(plugin);
  let symbols = run_logged(nm, staging, "verify-plugin-exports")?;
  ensure!(
    symbols.status.success(),
    "nm could not inspect rules plugin"
  );
  let symbols_text = String::from_utf8_lossy(&symbols.stdout);
  let symbols = symbols_text
    .lines()
    .map(|line| line.trim().trim_start_matches('_'))
    .collect::<std::collections::BTreeSet<_>>();
  let missing = REQUIRED_PLUGIN_EXPORTS
    .iter()
    .filter(|symbol| !symbols.contains(**symbol))
    .copied()
    .collect::<Vec<_>>();
  ensure!(
    missing.is_empty(),
    "rules plugin is missing required exports: {}",
    missing.join(", ")
  );
  Ok(())
}

fn clone_tree(source: &Path, destination: &Path) -> Result<()> {
  ensure!(
    source.is_dir(),
    "artifact is not a directory: {}",
    source.display()
  );
  let output = Command::new("/bin/cp")
    .args(["-cR"])
    .arg(source)
    .arg(destination)
    .output()
    .context("clone immutable build artifact")?;
  ensure!(
    output.status.success(),
    "clone build artifact: {}",
    String::from_utf8_lossy(&output.stderr)
  );
  Ok(())
}

fn failed(
  pending: PendingBuild,
  phase: &str,
  output: &Output,
  now: u64,
) -> Result<MacosBuildFailure> {
  let text = fs::read_to_string(pending.path().join(BUILD_LOG_FILE))?;
  failed_with_ids(
    pending,
    phase,
    format!("{phase} build exited with {}", output.status),
    error_ids(&text),
    now,
  )
}

fn failed_message(
  pending: PendingBuild,
  phase: &str,
  message: &str,
  now: u64,
) -> Result<MacosBuildFailure> {
  append_log(pending.path(), format!("{message}\n").as_bytes())?;
  failed_with_ids(pending, phase, message.to_owned(), Vec::new(), now)
}

fn failed_with_ids(
  pending: PendingBuild,
  phase: &str,
  message: String,
  error_ids: Vec<String>,
  now: u64,
) -> Result<MacosBuildFailure> {
  let identity = pending.identity().clone();
  let retained = BuildFailure {
    phase: phase.to_owned(),
    error_ids: error_ids.clone(),
    message: message.clone(),
    failed_at_unix_s: now,
  };
  let failure_path = pending.fail(&retained)?;
  Ok(MacosBuildFailure {
    identity,
    phase: phase.to_owned(),
    error_ids,
    message,
    log_path: failure_path.join(BUILD_LOG_FILE),
  })
}

fn run_logged(mut command: Command, staging: &Path, phase: &str) -> Result<Output> {
  append_log(staging, format!("==> {phase}\n").as_bytes())?;
  let output = command
    .output()
    .with_context(|| format!("launch {phase} build"))?;
  append_log(staging, &output.stdout)?;
  append_log(staging, &output.stderr)?;
  Ok(output)
}

fn append_log(staging: &Path, bytes: &[u8]) -> Result<()> {
  OpenOptions::new()
    .append(true)
    .open(staging.join(BUILD_LOG_FILE))?
    .write_all(bytes)?;
  Ok(())
}

fn append_unity_log(staging: &Path, path: &Path) -> Result<()> {
  if path.is_file() {
    append_log(staging, &fs::read(path)?)?;
  }
  Ok(())
}

fn cleanup_file(path: &Path) -> Result<()> {
  if path.exists() {
    fs::remove_file(path)?;
  }
  Ok(())
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  Ok(bytes)
}

fn generated(name: &str, value: &str) -> GeneratedInput {
  GeneratedInput {
    generator: "battlement-macos-components".to_owned(),
    version: RECIPE_VERSION.to_owned(),
    name: name.to_owned(),
    bytes: value.as_bytes().to_vec(),
  }
}

fn recipe_input(component: &str) -> GeneratedInput {
  generated(&format!("{component}-recipe"), &recipe_digest())
}

fn recipe_digest() -> String {
  format!("{:x}", Sha256::digest(include_bytes!("macos_build.rs")))
}

fn unity_scene(request: &MacosBuildRequest) -> Result<String> {
  let scene = request.scene.strip_prefix(&request.unity_project)?;
  ensure!(!scene.as_os_str().is_empty(), "Unity scene path is empty");
  Ok(scene.to_string_lossy().replace('\\', "/"))
}

fn rust_target(architecture: &str) -> Result<&'static str> {
  match architecture {
    "aarch64" | "arm64" => Ok("aarch64-apple-darwin"),
    _ => anyhow::bail!("unsupported macOS architecture: {architecture}"),
  }
}

fn event(kind: &str, identity: &BuildIdentity, disposition: &str, cause: &str) {
  eprintln!(
    "artifact.resolve kind={kind} fingerprint={} disposition={disposition} cause={cause}",
    identity.fingerprint
  );
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
