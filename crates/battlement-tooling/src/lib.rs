//! Shared host and build tooling for Battlement developer commands.

pub mod discovery;
pub mod doctor;
pub mod host;
pub mod unity_lease;

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};

/// Finds the Git worktree containing `path`.
pub fn repository_root(path: &Path) -> Result<PathBuf> {
  let output = std::process::Command::new("git")
    .args(["rev-parse", "--show-toplevel"])
    .current_dir(path)
    .output()
    .context("failed to inspect the Git worktree")?;
  if !output.status.success() {
    bail!("{} is not inside a Git worktree", path.display());
  }
  PathBuf::from(String::from_utf8(output.stdout)?.trim())
    .canonicalize()
    .context("failed to resolve the Git worktree root")
}

/// Resolves a relative path while rejecting traversal outside `root`.
pub fn contained_path(root: &Path, base: &Path, path: &Path) -> Result<PathBuf> {
  if path.is_absolute() {
    bail!("repository path must be relative: {}", path.display());
  }
  let path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    base.join(path)
  };
  let path = resolve_nearest(&normalize(&path))?;
  if !path.starts_with(root) {
    bail!("path escapes repository root: {}", path.display());
  }
  Ok(path)
}

/// Resolves symlinks through the nearest existing parent of `path`.
pub fn resolve_nearest(path: &Path) -> Result<PathBuf> {
  let mut existing = path;
  let mut missing = Vec::new();
  while !existing.exists() {
    let name = existing
      .file_name()
      .ok_or_else(|| anyhow::anyhow!("path has no existing parent: {}", path.display()))?;
    missing.push(name.to_owned());
    existing = existing
      .parent()
      .ok_or_else(|| anyhow::anyhow!("path has no existing parent: {}", path.display()))?;
  }
  let mut resolved = existing
    .canonicalize()
    .with_context(|| format!("failed to resolve {}", existing.display()))?;
  for component in missing.iter().rev() {
    resolved.push(component);
  }
  Ok(resolved)
}

fn normalize(path: &Path) -> PathBuf {
  let mut normalized = PathBuf::new();
  for component in path.components() {
    match component {
      Component::CurDir => {}
      Component::ParentDir => {
        normalized.pop();
      }
      component => normalized.push(component.as_os_str()),
    }
  }
  normalized
}
