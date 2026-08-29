use std::{
  collections::{BTreeSet, VecDeque},
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use toml::Value;

pub(super) fn manifests(repository: &Path, initial: &Path) -> Result<Vec<PathBuf>> {
  let mut pending = VecDeque::from([initial.canonicalize()?]);
  let mut manifests = BTreeSet::new();
  while let Some(manifest) = pending.pop_front() {
    ensure!(
      manifest.starts_with(repository),
      "Cargo manifest escapes repository"
    );
    if !manifests.insert(manifest.clone()) {
      continue;
    }
    let source =
      fs::read_to_string(&manifest).with_context(|| format!("read {}", manifest.display()))?;
    let value: Value =
      toml::from_str(&source).with_context(|| format!("parse {}", manifest.display()))?;
    let workspace = workspace_manifest(repository, &manifest)?;
    for dependency in dependencies(&value) {
      let path = match &dependency {
        Dependency::Path(path) => Some(path.clone()),
        Dependency::Workspace(name) => workspace
          .as_ref()
          .and_then(|(_, value)| workspace_dependency(value, name)),
      };
      let Some(path) = path else {
        continue;
      };
      let base = match &dependency {
        Dependency::Path(_) => manifest.parent().expect("manifest has a parent"),
        Dependency::Workspace(_) => workspace
          .as_ref()
          .and_then(|(path, _)| path.parent())
          .expect("workspace manifest has a parent"),
      };
      let package = base.join(path);
      ensure!(
        !fs::symlink_metadata(&package)?.file_type().is_symlink(),
        "local Cargo package is a directory symlink"
      );
      let manifest = package.join("Cargo.toml");
      ensure!(
        !fs::symlink_metadata(&manifest)?.file_type().is_symlink(),
        "local Cargo manifest is a symlink"
      );
      let local = manifest.canonicalize()?;
      ensure!(
        local.starts_with(repository),
        "local Cargo dependency escapes repository"
      );
      pending.push_back(local);
    }
  }
  Ok(manifests.into_iter().collect())
}

pub(super) fn applicable_support_files(repository: &Path, manifests: &[PathBuf]) -> Vec<PathBuf> {
  let mut support = BTreeSet::new();
  for manifest in manifests {
    let mut current = manifest.parent();
    while let Some(directory) = current {
      for relative in ["Cargo.lock", ".cargo/config", ".cargo/config.toml"] {
        let path = directory.join(relative);
        if path.is_file() {
          support.insert(path);
        }
      }
      if directory == repository {
        break;
      }
      current = directory.parent();
    }
  }
  support.into_iter().collect()
}

#[derive(Clone, Debug)]
enum Dependency {
  Path(String),
  Workspace(String),
}

fn dependencies(value: &Value) -> Vec<Dependency> {
  let mut found = Vec::new();
  collect_dependencies(value, None, &mut found);
  found
}

fn collect_dependencies(value: &Value, key: Option<&str>, found: &mut Vec<Dependency>) {
  let Value::Table(table) = value else {
    return;
  };
  if key.is_some_and(|key| key.ends_with("dependencies")) {
    for (name, specification) in table {
      let Value::Table(specification) = specification else {
        continue;
      };
      if let Some(path) = specification.get("path").and_then(Value::as_str) {
        found.push(Dependency::Path(path.to_owned()));
      } else if specification
        .get("workspace")
        .and_then(Value::as_bool)
        .unwrap_or(false)
      {
        found.push(Dependency::Workspace(name.clone()));
      }
    }
    return;
  }
  for (key, nested) in table {
    collect_dependencies(nested, Some(key), found);
  }
}

fn workspace_manifest(repository: &Path, manifest: &Path) -> Result<Option<(PathBuf, Value)>> {
  let mut current = manifest.parent();
  while let Some(directory) = current {
    let candidate = directory.join("Cargo.toml");
    if candidate.is_file() {
      let value: Value = toml::from_str(&fs::read_to_string(&candidate)?)?;
      if value.get("workspace").is_some() {
        return Ok(Some((candidate, value)));
      }
    }
    if directory == repository {
      break;
    }
    current = directory.parent();
  }
  Ok(None)
}

fn workspace_dependency(value: &Value, name: &str) -> Option<String> {
  value
    .get("workspace")?
    .get("dependencies")?
    .get(name)?
    .get("path")?
    .as_str()
    .map(str::to_owned)
}
