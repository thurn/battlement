use std::{
  env, fs,
  net::TcpStream,
  path::{Path, PathBuf},
  process::{Child, Command, ExitStatus},
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use tempfile::{Builder, TempDir};

use crate::{interrupted, plugin_build, reset_interrupted, tools};

struct SampleConfig {
  application: String,
  scene: String,
}

struct ProjectState {
  _backup: TempDir,
  paths: Vec<SavedPath>,
  restored: bool,
}

struct SavedPath {
  path: PathBuf,
  backup: Option<PathBuf>,
}

const DEFAULT_WEB_PORT: u16 = 8000;

pub(crate) fn build(name: &str, web: bool, web_threads: bool, release: bool) -> Result<PathBuf> {
  reset_interrupted();
  self::validate_name(name)?;
  let root = self::repository_root(name)?;
  let project = root.join("samples").join(name);
  let config = self::sample_config(&project)?;
  let manifest = project.join("rules/Cargo.toml");
  let package = tools::rules_package(&manifest)?;
  let editor = tools::unity_editor(&project)?;
  let web_build = if web_threads {
    plugin_build::WebBuild::Threaded
  } else {
    plugin_build::WebBuild::Compatible
  };
  let (plugin, plugin_directory, plugin_name) = if web {
    (
      plugin_build::web_rules_plugin(&package, release, &manifest, &editor, web_build)?,
      project.join("Assets/Plugins/WebGL"),
      "libbattlement_rules.a",
    )
  } else {
    let architecture = tools::host_architecture()?;
    (
      plugin_build::rules_plugin(&package, &[architecture], release, Some(&manifest))?,
      project.join(self::native_plugin_directory()),
      self::native_plugin_name(),
    )
  };
  fs::create_dir_all(&plugin_directory)
    .with_context(|| format!("failed to create {}", plugin_directory.display()))?;
  fs::copy(&plugin, plugin_directory.join(plugin_name))
    .context("failed to stage the sample native plugin")?;

  let profile = if release { "release" } else { "debug" };
  let output = if web {
    project
      .join("Build")
      .join(profile)
      .join(self::web_output_name(web_threads))
  } else {
    project
      .join("Build")
      .join(profile)
      .join(self::native_output_name(&config.application))
  };
  fs::create_dir_all(output.parent().expect("application has a build directory"))?;
  if interrupted() {
    bail!("sample build interrupted");
  }
  let mut state = ProjectState::capture(&project)?;
  let unity_log = Builder::new()
    .prefix("battlement-sample-build.")
    .tempfile()
    .context("failed to create the Unity sample build log")?;
  let mut command = Command::new(editor);
  let mut child = command
    .args([
      "-batchmode",
      "-nographics",
      "--burst-disable-compilation",
      "-quit",
      "-projectPath",
    ])
    .arg(&project)
    .args([
      "-buildTarget",
      if web {
        "WebGL"
      } else {
        self::native_build_target()
      },
    ])
    .args([
      "-executeMethod",
      "Battlement.Editor.BattlementSampleBuild.Build",
      "-logFile",
    ])
    .arg(unity_log.path())
    .env("BATTLEMENT_SAMPLE_BUILD_PATH", &output)
    .env("BATTLEMENT_SAMPLE_SCENE_PATH", &config.scene)
    .env(
      "BATTLEMENT_SAMPLE_PLATFORM",
      if web { "web" } else { "native" },
    )
    .env(
      "BATTLEMENT_SAMPLE_WEB_THREADS",
      if web_threads { "1" } else { "0" },
    )
    .env("BATTLEMENT_SAMPLE_RELEASE", if release { "1" } else { "0" })
    .spawn()
    .context("failed to launch Unity")?;
  let status = self::wait_for_child(&mut child).context("failed to wait for Unity")?;
  state.restore()?;
  if interrupted() {
    bail!("Unity sample build interrupted; restored the Unity project");
  }
  let log = fs::read_to_string(unity_log.path()).context("failed to read the Unity build log")?;
  if !status.success() {
    self::print_tail(&log, 120);
    bail!("Unity sample build exited with status {status}");
  }
  if !log.contains(&format!("BATTLEMENT_SAMPLE_BUILD_OK:{}", output.display())) {
    self::print_tail(&log, 120);
    bail!("Unity sample build omitted its success marker");
  }
  if web {
    self::configure_web_entry_point(&root, &output)?;
    if !output.join("index.html").is_file() {
      bail!(
        "sample Web build omitted {}",
        output.join("index.html").display()
      );
    }
    let build_directory = output.join("Build");
    let has_wasm = fs::read_dir(&build_directory)
      .with_context(|| format!("failed to inspect {}", build_directory.display()))?
      .filter_map(Result::ok)
      .any(|entry| entry.file_name().to_string_lossy().contains(".wasm"));
    if !has_wasm {
      bail!("sample Web build omitted its WebAssembly player");
    }
    if web_threads {
      self::validate_threaded_web_output(&output)?;
    }
  } else {
    let packaged_plugin = self::packaged_native_plugin(&output);
    if !packaged_plugin.is_file() {
      bail!("sample build omitted {}", packaged_plugin.display());
    }
    self::native_executable(&output)?;
  }
  fs::write(self::build_stamp(&output, web, web_threads), b"")
    .context("failed to record the completed sample build")?;
  println!("Built {}", output.display());
  Ok(output)
}

fn configure_web_entry_point(root: &Path, output: &Path) -> Result<()> {
  let index_path = output.join("index.html");
  let mut index = fs::read_to_string(&index_path)
    .with_context(|| format!("failed to read {}", index_path.display()))?;
  if !index.contains("autoSyncPersistentDataPath: true") {
    let marker = "var config = {";
    let Some(offset) = index.find(marker) else {
      bail!(
        "Web entry point {} has no Unity config object",
        index_path.display()
      );
    };
    let insertion = offset + marker.len();
    index.insert_str(insertion, "\n        autoSyncPersistentDataPath: true,");
  }
  if !index.contains("<script src=\"init.js\"></script>") {
    let Some(offset) = index.find("</head>") else {
      bail!(
        "Web entry point {} has no closing head",
        index_path.display()
      );
    };
    index.insert_str(offset, "  <script src=\"init.js\"></script>\n");
  }
  fs::write(&index_path, index)
    .with_context(|| format!("failed to configure {}", index_path.display()))?;
  fs::copy(root.join("web/init.js"), output.join("init.js"))
    .with_context(|| format!("failed to copy Web initializer into {}", output.display()))?;
  Ok(())
}

pub(crate) fn run(
  name: &str,
  web: bool,
  web_threads: bool,
  port: Option<u16>,
  release: bool,
) -> Result<()> {
  self::validate_name(name)?;
  let root = self::repository_root(name)?;
  let project = root.join("samples").join(name);
  let config = self::sample_config(&project)?;
  let profile = if release { "release" } else { "debug" };
  let existing = if web {
    project
      .join("Build")
      .join(profile)
      .join(self::web_output_name(web_threads))
  } else {
    project
      .join("Build")
      .join(profile)
      .join(self::native_output_name(&config.application))
  };
  let output = if existing.exists()
    && !self::requires_rebuild(&root, &project, &existing, web, web_threads)?
  {
    existing
  } else {
    self::build(name, web, web_threads, release)?
  };
  if web {
    return self::serve_web(&root, &output, port.unwrap_or(DEFAULT_WEB_PORT));
  }

  let executable = self::native_executable(&output)?;
  reset_interrupted();
  let mut player = Command::new(&executable)
    .args(["-logFile", "-"])
    .spawn()
    .with_context(|| format!("failed to run {}", executable.display()))?;
  let status = self::wait_for_child(&mut player)?;
  if interrupted() {
    return Ok(());
  }
  if !status.success() {
    bail!("sample player exited with status {status}");
  }
  Ok(())
}

fn requires_rebuild(
  root: &Path,
  project: &Path,
  output: &Path,
  web: bool,
  web_threads: bool,
) -> Result<bool> {
  let stamp = self::build_stamp(output, web, web_threads);
  let built_at = match fs::metadata(&stamp).and_then(|metadata| metadata.modified()) {
    Ok(modified) => modified,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
    Err(error) => return Err(error).context("failed to inspect the sample build stamp"),
  };
  let inputs = [
    root.join("Cargo.lock"),
    root.join("Cargo.toml"),
    root.join("Packages/com.battlement.client"),
    root.join("crates"),
    root.join("web/init.js"),
    project.join("Assets"),
    project.join("Packages"),
    project.join("ProjectSettings"),
    project.join("rules"),
    project.join("sample.toml"),
  ];
  for input in inputs {
    if self::modified_after(&input, built_at)? {
      return Ok(true);
    }
  }
  Ok(false)
}

fn build_stamp(output: &Path, web: bool, web_threads: bool) -> PathBuf {
  output
    .parent()
    .expect("sample application has a build directory")
    .join(if web_threads {
      ".battlement-web-threaded-build-stamp"
    } else if web {
      ".battlement-web-build-stamp"
    } else {
      ".battlement-build-stamp"
    })
}

fn serve_web(root: &Path, output: &Path, port: u16) -> Result<()> {
  let address = format!("127.0.0.1:{port}");
  if TcpStream::connect(&address).is_ok() {
    bail!("port {port} is already in use; select another with --port");
  }
  let python = env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
  reset_interrupted();
  let mut server = Command::new(python)
    .arg(root.join("scripts/serve_web.py"))
    .arg("--port")
    .arg(port.to_string())
    .arg("--directory")
    .arg(output)
    .spawn()
    .context("failed to start the local static server")?;
  if let Err(error) = self::wait_for_server(&mut server, &address) {
    self::stop_server(&mut server);
    if interrupted() {
      return Ok(());
    }
    return Err(error);
  }

  let url = format!("http://{address}/");
  println!("Running Battlement Web sample at {url}");
  println!("Press Ctrl-C to stop.");
  let open_status = match Command::new("open").arg(&url).status() {
    Ok(status) => status,
    Err(error) => {
      self::stop_server(&mut server);
      return Err(error).context("failed to open the Web sample in a browser");
    }
  };
  if !open_status.success() {
    self::stop_server(&mut server);
    bail!("browser opener exited with status {open_status}");
  }
  let status =
    self::wait_for_child(&mut server).context("failed to wait for the local static server")?;
  if interrupted() {
    return Ok(());
  }
  if !status.success() {
    bail!("local static server exited with status {status}");
  }
  Ok(())
}

fn wait_for_server(server: &mut Child, address: &str) -> Result<()> {
  let deadline = Instant::now() + Duration::from_secs(5);
  while Instant::now() < deadline {
    if let Some(status) = server.try_wait()? {
      bail!("local static server exited during startup with status {status}");
    }
    if TcpStream::connect(address).is_ok() {
      return Ok(());
    }
    thread::sleep(Duration::from_millis(50));
  }
  bail!("local static server did not listen on {address} within five seconds")
}

fn stop_server(server: &mut Child) {
  let _ = server.kill();
  let _ = server.wait();
}

fn wait_for_child(child: &mut Child) -> Result<ExitStatus> {
  loop {
    if let Some(status) = child.try_wait()? {
      return Ok(status);
    }
    if interrupted() {
      let _ = child.kill();
      return child.wait().context("failed to stop interrupted process");
    }
    thread::sleep(Duration::from_millis(50));
  }
}

fn validate_threaded_web_output(output: &Path) -> Result<()> {
  let index = fs::read_to_string(output.join("index.html"))
    .context("failed to inspect the threaded Web entry point")?;
  let external = [
    "src=\"http://",
    "src=\"https://",
    "href=\"http://",
    "href=\"https://",
  ];
  if external.iter().any(|value| index.contains(value)) {
    bail!("threaded Web entry point embeds a cross-origin resource");
  }
  Ok(())
}

fn modified_after(path: &Path, timestamp: std::time::SystemTime) -> Result<bool> {
  let metadata = fs::metadata(path)
    .with_context(|| format!("failed to inspect sample input {}", path.display()))?;
  if metadata.is_file() {
    return Ok(metadata.modified()? > timestamp);
  }
  for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
    if self::modified_after(&entry?.path(), timestamp)? {
      return Ok(true);
    }
  }
  Ok(false)
}

fn repository_root(name: &str) -> Result<PathBuf> {
  let mut directory = env::current_dir().context("failed to read the current directory")?;
  loop {
    let sample = directory.join("samples").join(name);
    if sample.join("ProjectSettings/ProjectVersion.txt").is_file()
      && sample.join("rules/Cargo.toml").is_file()
    {
      return Ok(directory);
    }
    if !directory.pop() {
      bail!("sample {name:?} was not found below any parent samples/ directory");
    }
  }
}

fn sample_config(project: &Path) -> Result<SampleConfig> {
  let path = project.join("sample.toml");
  let contents =
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
  Ok(SampleConfig {
    application: self::config_value(&contents, "application")?,
    scene: self::config_value(&contents, "scene")?,
  })
}

fn native_executable(application: &Path) -> Result<PathBuf> {
  #[cfg(windows)]
  {
    if !application.is_file() {
      bail!("sample build omitted {}", application.display());
    }
    Ok(application.to_owned())
  }
  #[cfg(target_os = "macos")]
  let info = application.join("Contents/Info.plist");
  #[cfg(target_os = "macos")]
  let result = Command::new("plutil")
    .args(["-extract", "CFBundleExecutable", "raw"])
    .arg(&info)
    .output()
    .with_context(|| format!("failed to inspect {}", info.display()))?;
  #[cfg(target_os = "macos")]
  if !result.status.success() {
    bail!(
      "failed to read CFBundleExecutable from {}: {}",
      info.display(),
      String::from_utf8_lossy(&result.stderr).trim()
    );
  }
  #[cfg(target_os = "macos")]
  let name = String::from_utf8(result.stdout)
    .context("application executable name is not UTF-8")?
    .trim()
    .to_owned();
  #[cfg(target_os = "macos")]
  if name.is_empty() || Path::new(&name).file_name() != Some(name.as_ref()) {
    bail!("application has an invalid executable name {name:?}");
  }
  #[cfg(target_os = "macos")]
  let executable = application.join("Contents/MacOS").join(name);
  #[cfg(target_os = "macos")]
  if !executable.is_file() {
    bail!("sample build omitted {}", executable.display());
  }
  #[cfg(target_os = "macos")]
  Ok(executable)
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
    Path::new(application).with_extension("exe")
  } else {
    application.into()
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

fn web_output_name(web_threads: bool) -> &'static str {
  if web_threads { "WebThreads" } else { "Web" }
}

fn config_value(contents: &str, key: &str) -> Result<String> {
  contents
    .lines()
    .filter_map(|line| line.split_once('='))
    .find_map(|(candidate, value)| {
      (candidate.trim() == key).then(|| {
        value
          .trim()
          .strip_prefix('"')?
          .strip_suffix('"')
          .map(str::to_owned)
      })?
    })
    .with_context(|| format!("sample.toml has no quoted {key} value"))
}

impl ProjectState {
  fn capture(project: &Path) -> Result<Self> {
    let backup = tempfile::tempdir().context("failed to create Unity project backup")?;
    let paths = [
      "Assets/AddressableAssetsData",
      "Assets/DefaultVolumeProfile.asset",
      "Assets/DefaultVolumeProfile.asset.meta",
      "Assets/Generated",
      "Assets/Generated.meta",
      "Assets/Original",
      "Assets/Scenes.meta",
      "Assets/UniversalRenderPipelineGlobalSettings.asset",
      "Assets/UniversalRenderPipelineGlobalSettings.asset.meta",
      "Packages/packages-lock.json",
      "ProjectSettings/EditorBuildSettings.asset",
      "ProjectSettings/GraphicsSettings.asset",
      "ProjectSettings/ProjectAuditorSettings.asset",
      "ProjectSettings/ProjectSettings.asset",
      "ProjectSettings/ShaderGraphSettings.asset",
      "ProjectSettings/TimeManager.asset",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, relative)| {
      SavedPath::capture(
        project.join(relative),
        backup.path().join(index.to_string()),
      )
    })
    .collect::<Result<Vec<_>>>()?;
    Ok(Self {
      _backup: backup,
      paths,
      restored: false,
    })
  }

  fn restore(&mut self) -> Result<()> {
    let mut error = None;
    for path in &self.paths {
      if let Err(current) = path.restore()
        && error.is_none()
      {
        error = Some(current);
      }
    }
    if let Some(error) = error {
      return Err(error).context("failed to restore the Unity project after building");
    }
    self.restored = true;
    Ok(())
  }
}

impl Drop for ProjectState {
  fn drop(&mut self) {
    if !self.restored {
      let _ = self.restore();
    }
  }
}

impl SavedPath {
  fn capture(path: PathBuf, backup: PathBuf) -> Result<Self> {
    let backup = if path.exists() {
      copy_path(&path, &backup)?;
      Some(backup)
    } else {
      None
    };
    Ok(Self { path, backup })
  }

  fn restore(&self) -> Result<()> {
    remove_path(&self.path)?;
    if let Some(backup) = &self.backup {
      copy_path(backup, &self.path)?;
    }
    Ok(())
  }
}

fn copy_path(source: &Path, destination: &Path) -> Result<()> {
  if source.is_dir() {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
      let entry = entry?;
      copy_path(&entry.path(), &destination.join(entry.file_name()))?;
    }
  } else {
    fs::create_dir_all(destination.parent().expect("backup path has a parent"))?;
    fs::copy(source, destination)?;
  }
  Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
  if path.is_dir() {
    fs::remove_dir_all(path)?;
  } else if path.exists() {
    fs::remove_file(path)?;
  }
  Ok(())
}

fn print_tail(contents: &str, count: usize) {
  let lines = contents.lines().collect::<Vec<_>>();
  eprintln!("{}", lines[lines.len().saturating_sub(count)..].join("\n"));
}

fn validate_name(name: &str) -> Result<()> {
  if name.is_empty()
    || !name
      .bytes()
      .all(|character| character.is_ascii_alphanumeric() || matches!(character, b'-' | b'_'))
  {
    bail!("sample names may contain only ASCII letters, digits, '-' and '_'");
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sample_names_are_safe_path_components() {
    assert!(self::validate_name("basic").is_ok());
    assert!(self::validate_name("future-sample_2").is_ok());
    assert!(self::validate_name("../basic").is_err());
    assert!(self::validate_name("").is_err());
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn native_executable_comes_from_the_application_bundle() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let application = directory.path().join("Battlement UI Lab.app");
    let contents = application.join("Contents");
    let executable = contents.join("MacOS/Battlement UI Lab");
    fs::create_dir_all(executable.parent().expect("executable has a parent"))?;
    fs::write(
      contents.join("Info.plist"),
      r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Battlement UI Lab</string>
</dict></plist>"#,
    )?;
    fs::write(&executable, "player")?;

    assert_eq!(self::native_executable(&application)?, executable);
    Ok(())
  }

  #[cfg(target_os = "macos")]
  #[test]
  fn native_executable_must_exist_in_the_application_bundle() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let application = directory.path().join("Sample.app");
    let contents = application.join("Contents");
    fs::create_dir_all(&contents)?;
    fs::write(
      contents.join("Info.plist"),
      r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>Missing Player</string>
</dict></plist>"#,
    )?;

    let error = self::native_executable(&application).unwrap_err();
    assert!(error.to_string().contains("Contents/MacOS/Missing Player"));
    Ok(())
  }
  #[test]
  fn project_state_restores_user_files_and_removes_build_residue() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let project = directory.path();
    let addressables = project.join("Assets/AddressableAssetsData/group.asset");
    let default_volume = project.join("Assets/DefaultVolumeProfile.asset");
    let render_settings = project.join("Assets/UniversalRenderPipelineGlobalSettings.asset");
    let editor_settings = project.join("ProjectSettings/EditorBuildSettings.asset");
    let graphics_settings = project.join("ProjectSettings/GraphicsSettings.asset");
    let auditor_settings = project.join("ProjectSettings/ProjectAuditorSettings.asset");
    let project_settings = project.join("ProjectSettings/ProjectSettings.asset");
    let shader_graph_settings = project.join("ProjectSettings/ShaderGraphSettings.asset");
    let packages_lock = project.join("Packages/packages-lock.json");
    let original_metadata = project.join("Assets/Original/source.svg.meta");
    for (path, contents) in [
      (&addressables, "user addressables\n"),
      (&default_volume, "user default volume\n"),
      (&render_settings, "user render settings\n"),
      (&editor_settings, "user editor settings\n"),
      (&graphics_settings, "user graphics settings\n"),
      (&auditor_settings, "user auditor settings\n"),
      (&project_settings, "user project settings\n"),
      (&shader_graph_settings, "user shader graph settings\n"),
      (&packages_lock, "user packages lock\n"),
      (&original_metadata, "user importer metadata\n"),
    ] {
      fs::create_dir_all(path.parent().expect("fixture file has a parent"))?;
      fs::write(path, contents)?;
    }

    let mut state = ProjectState::capture(project)?;
    fs::write(&addressables, "temporary addressables\n")?;
    fs::write(&default_volume, "temporary default volume\n")?;
    fs::write(&render_settings, "temporary render settings\n")?;
    fs::write(&editor_settings, "temporary editor settings\n")?;
    fs::write(&graphics_settings, "temporary graphics settings\n")?;
    fs::write(&auditor_settings, "temporary auditor settings\n")?;
    fs::write(&project_settings, "temporary project settings\n")?;
    fs::write(&shader_graph_settings, "temporary shader graph settings\n")?;
    fs::write(&packages_lock, "temporary packages lock\n")?;
    fs::write(&original_metadata, "temporary importer metadata\n")?;
    let generated_scenes_meta = project.join("Assets/Scenes.meta");
    let generated_render_settings_meta =
      project.join("Assets/UniversalRenderPipelineGlobalSettings.asset.meta");
    fs::write(&generated_scenes_meta, "temporary scenes metadata\n")?;
    fs::write(
      &generated_render_settings_meta,
      "temporary render settings metadata\n",
    )?;
    let generated = project.join("Assets/Generated/BattlementOpus/track.wav");
    fs::create_dir_all(generated.parent().expect("fixture file has a parent"))?;
    fs::write(&generated, "temporary audio")?;
    fs::write(
      project.join("Assets/Generated/BattlementOpus.meta"),
      "temporary metadata\n",
    )?;

    state.restore()?;

    assert_eq!(fs::read_to_string(addressables)?, "user addressables\n");
    assert_eq!(fs::read_to_string(default_volume)?, "user default volume\n");
    assert_eq!(
      fs::read_to_string(render_settings)?,
      "user render settings\n"
    );
    assert_eq!(
      fs::read_to_string(editor_settings)?,
      "user editor settings\n"
    );
    assert_eq!(
      fs::read_to_string(graphics_settings)?,
      "user graphics settings\n"
    );
    assert_eq!(
      fs::read_to_string(auditor_settings)?,
      "user auditor settings\n"
    );
    assert_eq!(
      fs::read_to_string(project_settings)?,
      "user project settings\n"
    );
    assert_eq!(
      fs::read_to_string(shader_graph_settings)?,
      "user shader graph settings\n"
    );
    assert_eq!(fs::read_to_string(packages_lock)?, "user packages lock\n");
    assert_eq!(
      fs::read_to_string(original_metadata)?,
      "user importer metadata\n"
    );
    assert!(!generated_scenes_meta.exists());
    assert!(!generated_render_settings_meta.exists());
    assert!(!project.join("Assets/Generated").exists());
    assert!(!project.join("Assets/Generated.meta").exists());
    Ok(())
  }

  #[test]
  fn interrupted_child_is_stopped() -> Result<()> {
    reset_interrupted();
    #[cfg(target_os = "windows")]
    let mut child = Command::new("powershell.exe")
      .args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"])
      .spawn()?;
    #[cfg(not(target_os = "windows"))]
    let mut child = Command::new("sh").args(["-c", "sleep 30"]).spawn()?;
    crate::INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst);

    let status = self::wait_for_child(&mut child)?;

    reset_interrupted();
    assert!(!status.success());
    assert!(child.try_wait()?.is_some());
    Ok(())
  }

  #[test]
  fn web_builds_enable_persistence_and_storage_reset() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("output");
    let web = temporary.path().join("web");
    fs::create_dir_all(&output).unwrap();
    fs::create_dir_all(&web).unwrap();
    fs::write(web.join("init.js"), "// Browser initializer.\n").unwrap();
    let index = output.join("index.html");
    fs::write(
            &index,
            "<html><head></head><body><script>\nvar config = {\n  productName: 'chess',\n};\n</script></body></html>",
        )
        .unwrap();

    self::configure_web_entry_point(temporary.path(), &output).unwrap();
    self::configure_web_entry_point(temporary.path(), &output).unwrap();

    let generated = fs::read_to_string(index).unwrap();
    assert_eq!(
      generated
        .matches("autoSyncPersistentDataPath: true")
        .count(),
      1
    );
    assert!(generated.contains("var config = {\n        autoSyncPersistentDataPath: true,"));
    assert_eq!(
      generated
        .matches("<script src=\"init.js\"></script>")
        .count(),
      1
    );
    assert_eq!(
      fs::read_to_string(output.join("init.js")).unwrap(),
      "// Browser initializer.\n"
    );
  }
}
