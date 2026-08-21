use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};

use crate::plugin_build;

struct SampleConfig {
    application: String,
    executable: String,
    scene: String,
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
        .args([
            "-executeMethod",
            "Masonry.Editor.MasonrySampleBuild.Build",
            "-logFile",
            "-",
        ])
        .env("MASONRY_SAMPLE_BUILD_PATH", &app)
        .env("MASONRY_SAMPLE_SCENE_PATH", &config.scene)
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
    fs::write(self::build_stamp(&app), b"")
        .context("failed to record the completed sample build")?;
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
    let app = if existing.is_dir() && !self::requires_rebuild(&root, &project, &existing)? {
        existing
    } else {
        self::build(name, release)?
    };
    let executable = app.join("Contents/MacOS").join(config.executable);
    let status = Command::new(&executable)
        .args(["-logFile", "-"])
        .status()
        .with_context(|| format!("failed to run {}", executable.display()))?;
    if !status.success() {
        bail!("sample player exited with status {status}");
    }
    Ok(())
}

fn requires_rebuild(root: &Path, project: &Path, app: &Path) -> Result<bool> {
    let stamp = self::build_stamp(app);
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

fn build_stamp(app: &Path) -> PathBuf {
    app.parent()
        .expect("sample application has a build directory")
        .join(".masonry-build-stamp")
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
