//! Strict authoring models for `ditto.toml` suites.

mod diagnostic;
pub mod model;
mod raw;
mod scenario;
mod validate;
pub mod value;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Loads and validates a full suite, discovering `ditto.toml` when needed.
pub fn load(explicit: Option<&Path>) -> Result<model::Suite> {
  let current = std::env::current_dir().context("failed to read the current directory")?;
  let source_path = match explicit {
    Some(path) if path.is_absolute() => path.to_path_buf(),
    Some(path) => current.join(path),
    None => discover(&current)?,
  };
  let source_path = source_path
    .canonicalize()
    .with_context(|| format!("failed to resolve suite {}", source_path.display()))?;
  let source = std::fs::read_to_string(&source_path)
    .with_context(|| format!("failed to read suite {}", source_path.display()))?;
  let parsed = toml::from_str(&source)
    .map_err(|error| diagnostic::parse_error(&source_path, &source, error))?;
  validate::suite(parsed, source_path, source).map_err(Into::into)
}

fn discover(start: &Path) -> Result<PathBuf> {
  start
    .ancestors()
    .map(|directory| directory.join("ditto.toml"))
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| anyhow::anyhow!("could not find ditto.toml from {}", start.display()))
}
