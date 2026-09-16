use std::{fs, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// Command-line project values that override `reactant.toml`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Overrides {
  /// Application product name.
  pub application: Option<String>,
  /// Cargo application-plugin manifest.
  pub manifest_path: Option<PathBuf>,
  /// Bootstrap scene.
  pub scene: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct Document {
  project: ProjectTable,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProjectTable {
  application: Option<String>,
  manifest_path: Option<PathBuf>,
  scene: Option<PathBuf>,
}

/// Resolves one explicit Reactant project and its application plugin.
pub fn resolve(
  root: PathBuf,
  overrides: Overrides,
) -> Result<battlement_tooling::application::Project> {
  let root = root
    .canonicalize()
    .with_context(|| format!("failed to locate Reactant project {}", root.display()))?;
  let configuration = root.join("reactant.toml");
  let contents = fs::read_to_string(&configuration)
    .with_context(|| format!("failed to read {}", configuration.display()))?;
  let document: Document = toml::from_str(&contents)
    .with_context(|| format!("failed to parse {}", configuration.display()))?;
  let application = overrides
    .application
    .or(document.project.application)
    .context("reactant.toml [project] requires application")?;
  if application.trim().is_empty() {
    bail!("reactant.toml [project] application must not be empty");
  }
  let scene = resolve_path(
    &root,
    overrides
      .scene
      .or(document.project.scene)
      .context("reactant.toml [project] requires scene")?,
  );
  let manifest = resolve_path(
    &root,
    overrides
      .manifest_path
      .or(document.project.manifest_path)
      .unwrap_or_else(|| PathBuf::from("rules/Cargo.toml")),
  );
  Ok(battlement_tooling::application::Project {
    root,
    application,
    scene,
    manifest,
  })
}

/// Resolves an invocation path from a Reactant project root.
pub fn resolve_output(root: &std::path::Path, output: Option<PathBuf>) -> Option<PathBuf> {
  output.map(|path| resolve_path(root, path))
}

fn resolve_path(root: &std::path::Path, path: PathBuf) -> PathBuf {
  if path.is_absolute() {
    path
  } else {
    root.join(path)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn external_project_with_spaces_uses_root_relative_defaults() -> Result<()> {
    let directory = tempfile::Builder::new().prefix("Card Table ").tempdir()?;
    fs::create_dir_all(directory.path().join("Assets/Scenes"))?;
    fs::create_dir_all(directory.path().join("rules"))?;
    fs::write(
      directory.path().join("reactant.toml"),
      "[project]\napplication = \"Card Table\"\nscene = \"Assets/Scenes/CardTable.unity\"\n",
    )?;

    let project = resolve(directory.path().to_owned(), Overrides::default())?;
    let root = directory.path().canonicalize()?;

    assert_eq!(project.application, "Card Table");
    assert_eq!(project.scene, root.join("Assets/Scenes/CardTable.unity"));
    assert_eq!(project.manifest, root.join("rules/Cargo.toml"));
    Ok(())
  }

  #[test]
  fn explicit_values_override_file_values_and_resolve_from_root() -> Result<()> {
    let directory = tempfile::tempdir()?;
    fs::write(
      directory.path().join("reactant.toml"),
      "[project]\napplication = \"Stored\"\nscene = \"Assets/Stored.unity\"\nmanifest-path = \"stored/Cargo.toml\"\n",
    )?;

    let project = resolve(
      directory.path().to_owned(),
      Overrides {
        application: Some("Override".to_owned()),
        manifest_path: Some("custom/plugin.toml".into()),
        scene: Some("Assets/Override.unity".into()),
      },
    )?;
    let root = directory.path().canonicalize()?;

    assert_eq!(project.application, "Override");
    assert_eq!(project.scene, root.join("Assets/Override.unity"));
    assert_eq!(project.manifest, root.join("custom/plugin.toml"));
    Ok(())
  }

  #[test]
  fn invocation_output_is_not_read_from_project_configuration() -> Result<()> {
    let directory = tempfile::tempdir()?;
    fs::write(
      directory.path().join("reactant.toml"),
      "[project]\napplication = \"Card Table\"\nscene = \"Assets/Main.unity\"\noutput = \"ignored\"\nweb = true\n",
    )?;

    let project = resolve(directory.path().to_owned(), Overrides::default())?;
    let root = directory.path().canonicalize()?;

    assert_eq!(resolve_output(&project.root, None), None);
    assert_eq!(
      resolve_output(&project.root, Some("chosen output".into())),
      Some(root.join("chosen output"))
    );
    Ok(())
  }
}
