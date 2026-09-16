use std::{
  fs,
  hash::{DefaultHasher, Hash, Hasher},
  path::{Path, PathBuf},
  process::{Child, Command, ExitStatus},
  sync::atomic::{AtomicBool, Ordering},
  thread,
  time::Duration,
};

use crate::unity_lease::{NativePlayerCapacityLease, UnityEditorLease};
use anyhow::{Context, Result, bail};
use tempfile::Builder;

const DEFAULT_WEB_PORT: u16 = 8000;
const WEB_INITIALIZER: &str = include_str!("../assets/init.js");

/// Inputs for building one Unity application and its Rust plugin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
  /// Unity project root.
  pub root: PathBuf,
  /// Player product name.
  pub application: String,
  /// Scene path below the Unity project.
  pub scene: PathBuf,
  /// Cargo manifest for the application plugin.
  pub manifest: PathBuf,
}

/// Per-invocation player build choices.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildOptions {
  /// Explicit output path, or the conventional project-local build path.
  pub output: Option<PathBuf>,
  /// Build the Rust plugin with the release profile.
  pub release: bool,
  /// Build a browser player instead of a native player.
  pub web: bool,
}

/// Builds one application after its owner prepares generated assets.
pub fn build<P>(
  project: &Project,
  options: &BuildOptions,
  interrupted: &AtomicBool,
  prepare: P,
) -> Result<PathBuf>
where
  P: FnOnce(&Path, &Path) -> Result<()>,
{
  validate(project)?;
  prepare(&project.root, &project.manifest).context("Reactant asset preparation failed")?;
  let package = crate::developer_tools::rules_package(&project.manifest)?;
  let editor = crate::developer_tools::unity_editor(&project.root)?;
  let (plugin, plugin_directory, plugin_name) = if options.web {
    (
      crate::plugin_build::web_rules_plugin(&package, options.release, &project.manifest, &editor)?,
      project.root.join("Assets/Plugins/WebGL"),
      "libbattlement_rules.a",
    )
  } else {
    let architecture = crate::developer_tools::host_architecture()?;
    (
      crate::plugin_build::rules_plugin(
        &package,
        &[architecture],
        options.release,
        Some(&project.manifest),
      )?,
      project.root.join(native_plugin_directory()),
      native_plugin_name(),
    )
  };
  fs::create_dir_all(&plugin_directory)
    .with_context(|| format!("failed to create {}", plugin_directory.display()))?;
  fs::copy(&plugin, plugin_directory.join(plugin_name))
    .context("failed to stage the application plugin")?;

  let output = output_path(project, options);
  fs::create_dir_all(
    output
      .parent()
      .context("application output has no parent directory")?,
  )?;
  if interrupted.load(Ordering::SeqCst) {
    bail!("application build interrupted");
  }
  let unity_log = Builder::new()
    .prefix("reactant-application-build.")
    .tempfile()
    .context("failed to create the Unity application build log")?;
  let capacity = UnityEditorLease::acquire(&crate::developer_tools::resource_slots())?;
  let mut command = crate::transactional_unity_command(&project.root, &editor)?;
  let mut child = command
    .args([
      "-batchmode",
      "-nographics",
      "--burst-disable-compilation",
      "-quit",
      "-projectPath",
    ])
    .arg(&project.root)
    .args([
      "-buildTarget",
      if options.web {
        "WebGL"
      } else {
        native_build_target()
      },
      "-executeMethod",
      "Battlement.Editor.BattlementSampleBuild.Build",
      "-logFile",
    ])
    .arg(unity_log.path())
    .env("BATTLEMENT_SAMPLE_BUILD_PATH", &output)
    .env("BATTLEMENT_SAMPLE_SCENE_PATH", scene_path(project)?)
    .env(
      "BATTLEMENT_SAMPLE_PLATFORM",
      if options.web { "web" } else { "native" },
    )
    .env(
      "BATTLEMENT_SAMPLE_RELEASE",
      if options.release { "1" } else { "0" },
    )
    .spawn()
    .context("failed to launch Unity")?;
  let status = wait_for_child(&mut child, interrupted).context("failed to wait for Unity")?;
  drop(capacity);
  if interrupted.load(Ordering::SeqCst) {
    bail!("Unity application build interrupted");
  }
  let log = fs::read_to_string(unity_log.path()).context("failed to read the Unity build log")?;
  if !status.success() {
    print_tail(&log, 120);
    bail!("Unity application build exited with status {status}");
  }
  if !log.contains(&format!("BATTLEMENT_SAMPLE_BUILD_OK:{}", output.display())) {
    print_tail(&log, 120);
    bail!("Unity application build omitted its success marker");
  }
  validate_output(&output, options.web)?;
  println!("Built {}", output.display());
  Ok(output)
}

/// Builds and launches one native or browser application.
pub fn run<P>(
  project: &Project,
  options: &BuildOptions,
  port: Option<u16>,
  interrupted: &AtomicBool,
  prepare: P,
) -> Result<()>
where
  P: FnOnce(&Path, &Path) -> Result<()>,
{
  run_after_build(
    || self::build(project, options, interrupted, prepare),
    |output| {
      if options.web {
        crate::application_web::serve(output, port.unwrap_or(DEFAULT_WEB_PORT), interrupted)
      } else {
        launch_native(output, interrupted)
      }
    },
  )
}

fn validate(project: &Project) -> Result<()> {
  for relative in [
    "Assets",
    "Packages/manifest.json",
    "ProjectSettings/ProjectVersion.txt",
  ] {
    if !project.root.join(relative).exists() {
      bail!(
        "{} is not a Unity project: {relative} is missing",
        project.root.display()
      );
    }
  }
  if project.application.trim().is_empty() {
    bail!("application name must not be empty");
  }
  let application = Path::new(&project.application);
  if application.file_name() != Some(application.as_os_str()) {
    bail!("application must be a product name, not a path");
  }
  if !project.manifest.is_file() {
    bail!(
      "application plugin manifest does not exist: {}",
      project.manifest.display()
    );
  }
  scene_path(project)?;
  Ok(())
}

fn run_after_build<B, L>(build: B, launch: L) -> Result<()>
where
  B: FnOnce() -> Result<PathBuf>,
  L: FnOnce(&Path) -> Result<()>,
{
  let output = build()?;
  launch(&output)
}

fn scene_path(project: &Project) -> Result<PathBuf> {
  let relative = project
    .scene
    .strip_prefix(&project.root)
    .with_context(|| {
      format!(
        "application scene {} is outside project {}",
        project.scene.display(),
        project.root.display()
      )
    })?
    .to_owned();
  let scene = project
    .scene
    .canonicalize()
    .with_context(|| format!("failed to locate scene {}", project.scene.display()))?;
  let assets = project.root.join("Assets").canonicalize()?;
  if !scene.starts_with(&assets)
    || scene.extension().and_then(|value| value.to_str()) != Some("unity")
  {
    bail!("application scene must be a .unity file below the project's Assets directory");
  }
  Ok(relative)
}

fn output_path(project: &Project, options: &BuildOptions) -> PathBuf {
  if let Some(output) = &options.output {
    return output.clone();
  }
  let profile = if options.release { "release" } else { "debug" };
  let name = if options.web {
    PathBuf::from("WebThreads")
  } else {
    native_output_name(&project.application)
  };
  project.root.join("Build").join(profile).join(name)
}

fn validate_output(output: &Path, web: bool) -> Result<()> {
  if web {
    configure_web_entry_point(output)?;
    if !output.join("index.html").is_file() {
      bail!("Web build omitted {}", output.join("index.html").display());
    }
    let build_directory = output.join("Build");
    let has_wasm = fs::read_dir(&build_directory)
      .with_context(|| format!("failed to inspect {}", build_directory.display()))?
      .filter_map(Result::ok)
      .any(|entry| entry.file_name().to_string_lossy().contains(".wasm"));
    if !has_wasm {
      bail!("Web build omitted its WebAssembly player");
    }
    validate_threaded_web_output(output)
  } else {
    let packaged_plugin = packaged_native_plugin(output);
    if !packaged_plugin.is_file() {
      bail!("application build omitted {}", packaged_plugin.display());
    }
    native_executable(output).map(|_| ())
  }
}

fn configure_web_entry_point(output: &Path) -> Result<()> {
  let index_path = output.join("index.html");
  let mut index = fs::read_to_string(&index_path)
    .with_context(|| format!("failed to read {}", index_path.display()))?;
  let mut fingerprint = DefaultHasher::new();
  WEB_INITIALIZER.hash(&mut fingerprint);
  let initializer_script = format!(
    "<script src=\"init.js?v={:016x}\"></script>",
    fingerprint.finish()
  );
  if !index.contains("autoSyncPersistentDataPath: true") {
    let marker = "var config = {";
    let offset = index
      .find(marker)
      .map(|offset| offset + marker.len())
      .with_context(|| {
        format!(
          "Web entry point {} has no Unity config object",
          index_path.display()
        )
      })?;
    index.insert_str(offset, "\n        autoSyncPersistentDataPath: true,");
  }
  if let Some(start) = index.find("<script src=\"init.js") {
    let end = index[start..]
      .find("</script>")
      .map(|offset| start + offset + "</script>".len())
      .context("Web entry point has an incomplete initializer script")?;
    index.replace_range(start..end, &initializer_script);
  } else {
    let offset = index.find("</head>").with_context(|| {
      format!(
        "Web entry point {} has no closing head",
        index_path.display()
      )
    })?;
    index.insert_str(offset, &format!("  {initializer_script}\n"));
  }
  fs::write(&index_path, index)
    .with_context(|| format!("failed to configure {}", index_path.display()))?;
  fs::write(output.join("init.js"), WEB_INITIALIZER)
    .with_context(|| format!("failed to write Web initializer into {}", output.display()))
}

fn launch_native(output: &Path, interrupted: &AtomicBool) -> Result<()> {
  let executable = native_executable(output)?;
  let _capacity = NativePlayerCapacityLease::acquire(&crate::developer_tools::resource_slots())?;
  let mut player = Command::new(&executable)
    .args(["-logFile", "-"])
    .spawn()
    .with_context(|| format!("failed to run {}", executable.display()))?;
  let status = wait_for_child(&mut player, interrupted)?;
  if !interrupted.load(Ordering::SeqCst) && !status.success() {
    bail!("application player exited with status {status}");
  }
  Ok(())
}

fn wait_for_child(child: &mut Child, interrupted: &AtomicBool) -> Result<ExitStatus> {
  loop {
    if let Some(status) = child.try_wait()? {
      return Ok(status);
    }
    if interrupted.load(Ordering::SeqCst) {
      let _ = child.kill();
      return child.wait().context("failed to stop interrupted process");
    }
    thread::sleep(Duration::from_millis(50));
  }
}

fn validate_threaded_web_output(output: &Path) -> Result<()> {
  let index = fs::read_to_string(output.join("index.html"))
    .context("failed to inspect the threaded Web entry point")?;
  if [
    "src=\"http://",
    "src=\"https://",
    "href=\"http://",
    "href=\"https://",
  ]
  .iter()
  .any(|value| index.contains(value))
  {
    bail!("threaded Web entry point embeds a cross-origin resource");
  }
  Ok(())
}

fn native_executable(application: &Path) -> Result<PathBuf> {
  #[cfg(windows)]
  {
    if !application.is_file() {
      bail!("application build omitted {}", application.display());
    }
    Ok(application.to_owned())
  }
  #[cfg(target_os = "macos")]
  {
    let info = application.join("Contents/Info.plist");
    let result = Command::new("plutil")
      .args(["-extract", "CFBundleExecutable", "raw"])
      .arg(&info)
      .output()
      .with_context(|| format!("failed to inspect {}", info.display()))?;
    if !result.status.success() {
      bail!(
        "failed to read CFBundleExecutable from {}: {}",
        info.display(),
        String::from_utf8_lossy(&result.stderr).trim()
      );
    }
    let name = String::from_utf8(result.stdout)
      .context("application executable name is not UTF-8")?
      .trim()
      .to_owned();
    if name.is_empty() || Path::new(&name).file_name() != Some(name.as_ref()) {
      bail!("application has an invalid executable name {name:?}");
    }
    let executable = application.join("Contents/MacOS").join(name);
    if !executable.is_file() {
      bail!("application build omitted {}", executable.display());
    }
    Ok(executable)
  }
}

fn native_plugin_directory() -> &'static str {
  if cfg!(windows) {
    "Assets/Plugins/x86_64"
  } else {
    "Assets/Plugins/macOS"
  }
}

fn native_plugin_name() -> &'static str {
  if cfg!(windows) {
    "battlement_rules.dll"
  } else {
    "libbattlement_rules.dylib"
  }
}

fn native_build_target() -> &'static str {
  if cfg!(windows) {
    "StandaloneWindows64"
  } else {
    "StandaloneOSX"
  }
}

fn native_output_name(application: &str) -> PathBuf {
  if cfg!(windows) {
    if application.ends_with(".exe") {
      application.into()
    } else {
      format!("{application}.exe").into()
    }
  } else if application.ends_with(".app") {
    application.into()
  } else {
    format!("{application}.app").into()
  }
}

fn packaged_native_plugin(output: &Path) -> PathBuf {
  if cfg!(windows) {
    output
      .parent()
      .expect("Windows player has a build directory")
      .join(format!(
        "{}_Data",
        output
          .file_stem()
          .expect("Windows player has a file stem")
          .to_string_lossy()
      ))
      .join("Plugins/x86_64/battlement_rules.dll")
  } else {
    output.join("Contents/PlugIns/libbattlement_rules.dylib")
  }
}

fn print_tail(contents: &str, count: usize) {
  let lines = contents.lines().collect::<Vec<_>>();
  eprintln!("{}", lines[lines.len().saturating_sub(count)..].join("\n"));
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn application_names_receive_platform_extensions_once() {
    let expected = if cfg!(windows) {
      PathBuf::from("Card Table.exe")
    } else {
      PathBuf::from("Card Table.app")
    };
    assert_eq!(native_output_name("Card Table"), expected);
    assert_eq!(native_output_name(expected.to_str().unwrap()), expected);
    let dotted = if cfg!(windows) {
      PathBuf::from("Card.Table.exe")
    } else {
      PathBuf::from("Card.Table.app")
    };
    assert_eq!(native_output_name("Card.Table"), dotted);
  }

  #[test]
  fn web_initializer_is_packaged_and_idempotent() -> Result<()> {
    let directory = tempfile::tempdir()?;
    fs::write(
      directory.path().join("index.html"),
      "<html><head></head><body><script>var config = {}</script></body></html>",
    )?;

    configure_web_entry_point(directory.path())?;
    configure_web_entry_point(directory.path())?;

    let index = fs::read_to_string(directory.path().join("index.html"))?;
    assert_eq!(index.matches("autoSyncPersistentDataPath: true").count(), 1);
    assert_eq!(index.matches("<script src=\"init.js?v=").count(), 1);
    assert_eq!(
      fs::read_to_string(directory.path().join("init.js"))?,
      WEB_INITIALIZER
    );
    Ok(())
  }

  #[test]
  fn failed_build_prevents_launch() {
    let launched = std::cell::Cell::new(false);

    let error = run_after_build(
      || bail!("asset preparation failed"),
      |_| {
        launched.set(true);
        Ok(())
      },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "asset preparation failed");
    assert!(!launched.get());
  }
}
