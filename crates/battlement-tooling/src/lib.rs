//! Shared host and build tooling for Battlement developer commands.

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
  let path = if path.is_absolute() {
    path.to_path_buf()
  } else {
    base.join(path)
  };
  let path = normalize(&path);
  if !path.starts_with(root) {
    bail!("path escapes repository root: {}", path.display());
  }
  Ok(path)
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
