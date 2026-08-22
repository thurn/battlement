use std::{
    env, fs,
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use tempfile::TempDir;

use crate::{interrupted, plugin_build, reset_interrupted};

struct SampleConfig {
    application: String,
    executable: String,
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
    let package = self::rules_package(&manifest)?;
    let editor = self::unity_editor(&project)?;
    let web_build = if web_threads {
        plugin_build::WebBuild::Threaded
    } else {
        plugin_build::WebBuild::Compatible
    };
    let (plugin, plugin_directory, plugin_name) = if web {
        (
            plugin_build::web_rules_plugin(&package, release, &manifest, &editor, web_build)?,
            project.join("Assets/Plugins/WebGL"),
            "libmasonry_rules.a",
        )
    } else {
        let architecture = self::host_architecture()?;
        (
            plugin_build::rules_plugin(&package, &[architecture], release, Some(&manifest))?,
            project.join("Assets/Plugins/macOS"),
            "libmasonry_rules.dylib",
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
            .join(&config.application)
    };
    fs::create_dir_all(output.parent().expect("application has a build directory"))?;
    if interrupted() {
        bail!("sample build interrupted");
    }
    let mut state = ProjectState::capture(&project)?;
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
        .args(["-buildTarget", if web { "WebGL" } else { "StandaloneOSX" }])
        .args([
            "-executeMethod",
            "Masonry.Editor.MasonrySampleBuild.Build",
            "-logFile",
            "-",
        ])
        .env("MASONRY_SAMPLE_BUILD_PATH", &output)
        .env("MASONRY_SAMPLE_SCENE_PATH", &config.scene)
        .env(
            "MASONRY_SAMPLE_PLATFORM",
            if web { "web" } else { "native" },
        )
        .env(
            "MASONRY_SAMPLE_WEB_THREADS",
            if web_threads { "1" } else { "0" },
        )
        .env("MASONRY_SAMPLE_RELEASE", if release { "1" } else { "0" })
        .spawn()
        .context("failed to launch Unity")?;
    let status = self::wait_for_child(&mut child).context("failed to wait for Unity")?;
    state.restore()?;
    if interrupted() {
        bail!("Unity sample build interrupted; restored the Unity project");
    }
    if !status.success() {
        bail!("Unity sample build exited with status {status}");
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
        let packaged_plugin = output.join("Contents/PlugIns/libmasonry_rules.dylib");
        if !packaged_plugin.is_file() {
            bail!("sample build omitted {}", packaged_plugin.display());
        }
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
        project.join("Build").join(profile).join(config.application)
    };
    let output = if existing.is_dir()
        && !self::requires_rebuild(&root, &project, &existing, web, web_threads)?
    {
        existing
    } else {
        self::build(name, web, web_threads, release)?
    };
    if web {
        return self::serve_web(&root, &output, port.unwrap_or(DEFAULT_WEB_PORT));
    }

    let executable = output.join("Contents/MacOS").join(config.executable);
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
        root.join("Packages/com.masonry.client"),
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
            ".masonry-web-threaded-build-stamp"
        } else if web {
            ".masonry-web-build-stamp"
        } else {
            ".masonry-build-stamp"
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
    println!("Running Masonry Web sample at {url}");
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

fn rules_package(manifest: &Path) -> Result<String> {
    let contents = fs::read_to_string(manifest)
        .with_context(|| format!("failed to read {}", manifest.display()))?;
    let name = contents
        .lines()
        .skip_while(|line| line.trim() != "[package]")
        .skip(1)
        .find_map(|line| line.trim().strip_prefix("name = \"")?.strip_suffix('"'))
        .context("sample rules manifest has no package name")?;
    Ok(name.to_owned())
}

fn sample_config(project: &Path) -> Result<SampleConfig> {
    let path = project.join("sample.toml");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(SampleConfig {
        application: self::config_value(&contents, "application")?,
        executable: self::config_value(&contents, "executable")?,
        scene: self::config_value(&contents, "scene")?,
    })
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

fn host_architecture() -> Result<String> {
    let output = Command::new("uname")
        .arg("-m")
        .output()
        .context("failed to determine the host architecture")?;
    if !output.status.success() {
        bail!("uname exited with status {}", output.status);
    }
    let architecture = String::from_utf8(output.stdout)?.trim().to_owned();
    match architecture.as_str() {
        "arm64" | "x86_64" => Ok(architecture),
        _ => bail!("unsupported macOS architecture: {architecture}"),
    }
}

impl ProjectState {
    fn capture(project: &Path) -> Result<Self> {
        let backup = tempfile::tempdir().context("failed to create Unity project backup")?;
        let paths = [
            "Assets/AddressableAssetsData",
            "Assets/Generated",
            "Assets/Generated.meta",
            "Assets/UniversalRenderPipelineGlobalSettings.asset",
            "ProjectSettings/EditorBuildSettings.asset",
            "ProjectSettings/ProjectSettings.asset",
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

fn unity_editor(project: &Path) -> Result<PathBuf> {
    if let Some(configured) = env::var_os("UNITY_EDITOR") {
        return Ok(configured.into());
    }
    let version = fs::read_to_string(project.join("ProjectSettings/ProjectVersion.txt"))?
        .lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion: "))
        .context("sample ProjectVersion.txt has no editor version")?
        .to_owned();
    Ok(format!("/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity").into())
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
    #[test]
    fn project_state_restores_user_files_and_removes_build_residue() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let project = directory.path();
        let addressables = project.join("Assets/AddressableAssetsData/group.asset");
        let render_settings = project.join("Assets/UniversalRenderPipelineGlobalSettings.asset");
        let editor_settings = project.join("ProjectSettings/EditorBuildSettings.asset");
        let project_settings = project.join("ProjectSettings/ProjectSettings.asset");
        for (path, contents) in [
            (&addressables, "user addressables\n"),
            (&render_settings, "user render settings\n"),
            (&editor_settings, "user editor settings\n"),
            (&project_settings, "user project settings\n"),
        ] {
            fs::create_dir_all(path.parent().expect("fixture file has a parent"))?;
            fs::write(path, contents)?;
        }

        let mut state = ProjectState::capture(project)?;
        fs::write(&addressables, "temporary addressables\n")?;
        fs::write(&render_settings, "temporary render settings\n")?;
        fs::write(&editor_settings, "temporary editor settings\n")?;
        fs::write(&project_settings, "temporary project settings\n")?;
        let generated = project.join("Assets/Generated/MasonryOpus/track.wav");
        fs::create_dir_all(generated.parent().expect("fixture file has a parent"))?;
        fs::write(&generated, "temporary audio")?;
        fs::write(
            project.join("Assets/Generated/MasonryOpus.meta"),
            "temporary metadata\n",
        )?;

        state.restore()?;

        assert_eq!(fs::read_to_string(addressables)?, "user addressables\n");
        assert_eq!(
            fs::read_to_string(render_settings)?,
            "user render settings\n"
        );
        assert_eq!(
            fs::read_to_string(editor_settings)?,
            "user editor settings\n"
        );
        assert_eq!(
            fs::read_to_string(project_settings)?,
            "user project settings\n"
        );
        assert!(!project.join("Assets/Generated").exists());
        assert!(!project.join("Assets/Generated.meta").exists());
        Ok(())
    }

    #[test]
    fn interrupted_child_is_stopped() -> Result<()> {
        reset_interrupted();
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
