use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus},
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};

use crate::{interrupted, plugin_build, reset_interrupted, tools};

const BOOTSTRAP_IDENTIFIER: &str = "Battlement.JSON::Battlement.BattlementBootstrap";

pub(crate) fn run(
    project: &Path,
    manifest_path: Option<&Path>,
    scene: Option<&Path>,
    release: bool,
) -> Result<()> {
    reset_interrupted();
    let project = project
        .canonicalize()
        .with_context(|| format!("failed to locate Unity project {}", project.display()))?;
    require_project(&project)?;
    let manifest = manifest_path
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join("rules/Cargo.toml"));
    let manifest = manifest.canonicalize().with_context(|| {
        format!(
            "failed to locate rules manifest {}; pass --manifest-path when it is not rules/Cargo.toml",
            manifest.display()
        )
    })?;
    let scene = match scene {
        Some(scene) => scene_path(&project, scene)?,
        None => detect_bootstrap_scene(&project)?,
    };
    let package = tools::rules_package(&manifest)?;
    let architecture = tools::host_architecture()?;
    let plugin = plugin_build::rules_plugin(&package, &[architecture], release, Some(&manifest))?;
    let destination = project.join("Assets/Plugins/macOS/libbattlement_rules.dylib");
    fs::create_dir_all(destination.parent().expect("plugin has a parent directory"))?;
    fs::copy(&plugin, &destination).context("failed to stage the native rules plugin")?;
    if interrupted() {
        bail!("authoring launch interrupted");
    }

    let mut child = Command::new(tools::unity_editor(&project)?)
        .args(["-projectPath"])
        .arg(&project)
        .args([
            "-executeMethod",
            "Battlement.Editor.BattlementAuthoring.OpenAndPlay",
            "-logFile",
            "-",
        ])
        .env("BATTLEMENT_AUTHOR_SCENE_PATH", &scene)
        .spawn()
        .context("failed to launch the Unity Editor")?;
    let status = wait_for_child(&mut child).context("failed to wait for the Unity Editor")?;
    if interrupted() {
        return Ok(());
    }
    if !status.success() {
        bail!("Unity Editor exited with status {status}");
    }
    Ok(())
}

fn require_project(project: &Path) -> Result<()> {
    for relative in [
        "Assets",
        "Packages/manifest.json",
        "ProjectSettings/ProjectVersion.txt",
    ] {
        if !project.join(relative).exists() {
            bail!(
                "{} is not a Unity project: {relative} is missing",
                project.display()
            );
        }
    }
    Ok(())
}

fn scene_path(project: &Path, scene: &Path) -> Result<PathBuf> {
    let path = if scene.is_absolute() {
        scene.to_owned()
    } else {
        project.join(scene)
    };
    let path = path
        .canonicalize()
        .with_context(|| format!("failed to locate scene {}", path.display()))?;
    if !path.starts_with(project.join("Assets"))
        || path.extension().and_then(|value| value.to_str()) != Some("unity")
    {
        bail!("authoring scene must be a .unity file below the project's Assets directory");
    }
    Ok(path
        .strip_prefix(project)
        .expect("validated scene belongs to the project")
        .to_owned())
}

fn detect_bootstrap_scene(project: &Path) -> Result<PathBuf> {
    let mut matches = Vec::new();
    collect_bootstrap_scenes(&project.join("Assets"), project, &mut matches)?;
    match matches.as_slice() {
        [scene] => Ok(scene.clone()),
        [] => {
            bail!("no scene with a BattlementBootstrap was found; pass --scene Assets/path.unity")
        }
        _ => bail!(
            "multiple scenes with BattlementBootstrap were found; pass --scene to select one: {}",
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn collect_bootstrap_scenes(
    directory: &Path,
    project: &Path,
    matches: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to inspect {}", directory.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_bootstrap_scenes(&path, project, matches)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("unity")
            && fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?
                .contains(BOOTSTRAP_IDENTIFIER)
        {
            matches.push(
                path.strip_prefix(project)
                    .expect("Assets directory belongs to the project")
                    .to_owned(),
            );
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_scene_is_discovered_in_an_external_unity_project() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let assets = directory.path().join("Assets/Scenes");
        fs::create_dir_all(&assets)?;
        fs::write(assets.join("Content.unity"), "ordinary content\n")?;
        fs::write(
            assets.join("Main.unity"),
            format!("m_EditorClassIdentifier: {BOOTSTRAP_IDENTIFIER}\n"),
        )?;

        assert_eq!(
            detect_bootstrap_scene(directory.path())?,
            PathBuf::from("Assets/Scenes/Main.unity")
        );
        Ok(())
    }

    #[test]
    fn ambiguous_bootstrap_scenes_require_an_explicit_choice() -> Result<()> {
        let directory = tempfile::tempdir()?;
        let assets = directory.path().join("Assets");
        fs::create_dir_all(&assets)?;
        fs::write(assets.join("First.unity"), BOOTSTRAP_IDENTIFIER)?;
        fs::write(assets.join("Second.unity"), BOOTSTRAP_IDENTIFIER)?;

        let error = detect_bootstrap_scene(directory.path()).unwrap_err();

        assert!(error.to_string().contains("multiple scenes"));
        assert!(error.to_string().contains("--scene"));
        Ok(())
    }
}
