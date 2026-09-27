use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::macos_build::{self, MacosBuildRequest};

pub(crate) struct ContentProject(PathBuf);

pub(crate) fn stage(request: &MacosBuildRequest, staging: &Path) -> Result<ContentProject> {
  let project = ContentProject(staging.join("content-project"));
  fs::create_dir_all(project.path())?;
  for directory in ["Assets", "Packages", "ProjectSettings", "Library"] {
    let source = request.unity_project.join(directory);
    if source.is_dir() {
      macos_build::clone_tree(&source, &project.path().join(directory))?;
    }
  }
  self::embed_client(request, project.path())?;
  Ok(project)
}

pub(crate) fn embed_client(request: &MacosBuildRequest, project: &Path) -> Result<()> {
  let embedded_package = project.join("Packages/com.battlement.client");
  if !embedded_package.is_dir() {
    macos_build::clone_tree(&macos_build::shell_package(request), &embedded_package)?;
  }
  let manifest_path = project.join("Packages/manifest.json");
  let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
  let manifest = manifest
    .as_object_mut()
    .context("player package manifest is not an object")?;
  manifest.remove("testables");
  manifest
    .get_mut("dependencies")
    .and_then(Value::as_object_mut)
    .context("player package dependencies are not an object")?
    .insert(
      "com.battlement.client".to_owned(),
      Value::String("file:com.battlement.client".to_owned()),
    );
  fs::write(&manifest_path, serde_json::to_vec(&manifest)?)?;
  let lock_path = project.join("Packages/packages-lock.json");
  if lock_path.is_file() {
    let mut lock: Value = serde_json::from_slice(&fs::read(&lock_path)?)?;
    if let Some(package) = lock["dependencies"]["com.battlement.client"].as_object_mut() {
      package.insert(
        "version".to_owned(),
        Value::String("file:com.battlement.client".to_owned()),
      );
    }
    fs::write(lock_path, serde_json::to_vec(&lock)?)?;
  }
  Ok(())
}

impl ContentProject {
  pub(crate) fn path(&self) -> &Path {
    &self.0
  }
}

impl Drop for ContentProject {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(self.path());
  }
}
