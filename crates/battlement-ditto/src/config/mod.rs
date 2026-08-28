//! Strict authoring models for `ditto.toml` suites.

mod diagnostic;
pub mod model;
mod raw;
mod scenario;
mod validate;
pub mod value;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

/// A file-backed or standard-input capture fragment.
#[derive(Clone, Debug)]
pub enum FragmentInput {
  File(PathBuf),
  StandardInput {
    source: String,
    name: Option<String>,
  },
}

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

/// Loads a full capture suite or resolves a fragment against `base`.
pub fn load_fragment(
  base: &model::Suite,
  input: FragmentInput,
  watch: bool,
) -> Result<model::Suite> {
  let (source_path, source, standard_input) = match input {
    FragmentInput::File(path) => {
      let path = if path.is_absolute() {
        path
      } else {
        std::env::current_dir()?.join(path)
      };
      let path = path
        .canonicalize()
        .with_context(|| format!("failed to resolve fragment {}", path.display()))?;
      let source = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read fragment {}", path.display()))?;
      (path, source, false)
    }
    FragmentInput::StandardInput { source, name } => {
      if watch {
        bail!("standard-input fragments do not support --watch");
      }
      let filename = name.unwrap_or_else(|| "standard-input".to_owned());
      (
        base.source.with_file_name(format!("<{filename}>")),
        source,
        true,
      )
    }
  };
  let document: toml::Table = toml::from_str(&source)
    .map_err(|error| diagnostic::parse_error(&source_path, &source, error))?;
  let full_suite = document.contains_key("player") || document.contains_key("profiles");
  if full_suite {
    let parsed = toml::from_str(&source)
      .map_err(|error| diagnostic::parse_error(&source_path, &source, error))?;
    return validate::suite(parsed, source_path, source).map_err(Into::into);
  }
  let parsed = toml::from_str(&source)
    .map_err(|error| diagnostic::parse_error(&source_path, &source, error))?;
  validate::fragment(parsed, base, source_path, source, standard_input).map_err(Into::into)
}

fn discover(start: &Path) -> Result<PathBuf> {
  start
    .ancestors()
    .map(|directory| directory.join("ditto.toml"))
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| anyhow::anyhow!("could not find ditto.toml from {}", start.display()))
}
