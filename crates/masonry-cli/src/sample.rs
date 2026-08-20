use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::plugin_build;

struct SampleConfig {
    application: String,
    build_method: String,
    capture_scene: String,
    capture_scenario: String,
    capture_task: String,
}

pub(crate) fn build(name: &str, release: bool) -> Result<PathBuf> {
    self::validate_name(name)?;
    let root = self::repository_root(name)?;
    let project = root.join("samples").join(name);
    let config = self::sample_config(&project)?;
    let manifest = project.join("rules/Cargo.toml");
    let package = self::rules_package(&manifest)?;
    let architecture = self::host_architecture()?;
    let plugin = plugin_build::rules_plugin(&package, &[architecture], release, Some(&manifest))?;
    let plugin_directory = project.join("Assets/Plugins/macOS");
    fs::create_dir_all(&plugin_directory)
        .with_context(|| format!("failed to create {}", plugin_directory.display()))?;
    fs::copy(&plugin, plugin_directory.join("libmasonry_rules.dylib"))
        .context("failed to stage the sample native plugin")?;

    let profile = if release { "release" } else { "debug" };
    let app = project
        .join("Build")
        .join(profile)
        .join(&config.application);
    fs::create_dir_all(app.parent().expect("application has a parent"))?;
    let status = Command::new(self::unity_editor(&project)?)
        .args([
            "-batchmode",
            "-nographics",
            "--burst-disable-compilation",
            "-quit",
            "-projectPath",
        ])
        .arg(&project)
        .args(["-executeMethod", &config.build_method, "-logFile", "-"])
        .env("MASONRY_SAMPLE_BUILD_PATH", &app)
        .env("MASONRY_SAMPLE_RELEASE", if release { "1" } else { "0" })
        .status()
        .context("failed to launch Unity")?;
    if !status.success() {
        bail!("Unity sample build exited with status {status}");
    }
    let packaged_plugin = app.join("Contents/PlugIns/libmasonry_rules.dylib");
    if !packaged_plugin.is_file() {
        bail!("sample build omitted {}", packaged_plugin.display());
    }
    if release {
        self::capture_release(&root, &project, &plugin, &config)?;
    }
    println!("Built {}", app.display());
    Ok(app)
}

pub(crate) fn run(name: &str, release: bool) -> Result<()> {
    self::validate_name(name)?;
    let root = self::repository_root(name)?;
    let project = root.join("samples").join(name);
    let config = self::sample_config(&project)?;
    let profile = if release { "release" } else { "debug" };
    let existing = project.join("Build").join(profile).join(config.application);
    let app = if existing.is_dir() {
        existing
    } else {
        self::build(name, release)?
    };
    let status = Command::new("open")
        .arg(&app)
        .status()
        .context("failed to open the sample application")?;
    if !status.success() {
        bail!("open exited with status {status}");
    }
    Ok(())
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
        build_method: self::config_value(&contents, "build-method")?,
        capture_scene: self::config_value(&contents, "capture-scene")?,
        capture_scenario: self::config_value(&contents, "capture-scenario")?,
        capture_task: self::config_value(&contents, "capture-task")?,
    })
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

fn capture_release(
    root: &Path,
    project: &Path,
    plugin: &Path,
    config: &SampleConfig,
) -> Result<()> {
    let status = Command::new("python3")
        .arg(root.join("scripts/capture-visual-evidence.py"))
        .args([
            "--task",
            &config.capture_task,
            "--scenario",
            &config.capture_scenario,
            "--scene",
            &config.capture_scene,
            "--transport",
            "native",
            "--capture",
            "both",
            "--input-driver",
            "in-player",
            "--media-driver",
            "screen-capture-kit",
            "--build-method",
            &config.build_method,
            "--dimensions",
            "1280x720",
            "--video-seconds",
            "7",
        ])
        .arg("--project-root")
        .arg(project)
        .arg("--plugin")
        .arg(plugin)
        .current_dir(root)
        .status()
        .context("failed to launch the visual capture workflow")?;
    if !status.success() {
        bail!("release sample walkthrough exited with status {status}");
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
}
