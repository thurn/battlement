//! Canonical `ditto.lock` documents and starting-state digests.

use std::{collections::BTreeSet, fs, io::ErrorKind, path::Path, str};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The generated index of accepted screenshots for one suite.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineManifest {
  pub suite: String,
  pub namespace: String,
  pub baselines: Vec<BaselineEntry>,
}

/// One accepted screenshot identity and its immutable PNG metadata.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineEntry {
  pub profile: String,
  pub scenario: String,
  pub checkpoint: String,
  pub sha256: String,
  pub width: u32,
  pub height: u32,
  pub size_bytes: u64,
  pub source: String,
}

/// A parsed lock and the digest of its exact starting bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSnapshot {
  pub manifest: Option<BaselineManifest>,
  pub sha256: Option<String>,
}

impl BaselineManifest {
  /// Parses and validates a lock document.
  pub fn parse(bytes: &[u8]) -> Result<Self> {
    let text = str::from_utf8(bytes).context("ditto.lock is not UTF-8")?;
    let manifest: Self = toml::from_str(text).context("parse ditto.lock")?;
    manifest.validate()?;
    Ok(manifest)
  }

  /// Produces deterministic TOML with sorted entries and one final newline.
  pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
    self.validate()?;
    let mut canonical = self.clone();
    canonical.baselines.sort_by(|left, right| {
      (&left.profile, &left.scenario, &left.checkpoint).cmp(&(
        &right.profile,
        &right.scenario,
        &right.checkpoint,
      ))
    });
    Ok(toml::to_string(&canonical)?.into_bytes())
  }

  /// Finds one accepted screenshot by its complete baseline identity.
  pub fn find(&self, profile: &str, scenario: &str, checkpoint: &str) -> Option<&BaselineEntry> {
    self.baselines.iter().find(|entry| {
      entry.profile == profile && entry.scenario == scenario && entry.checkpoint == checkpoint
    })
  }

  fn validate(&self) -> Result<()> {
    validate_name("suite", &self.suite)?;
    validate_namespace(&self.namespace)?;
    let mut identities = BTreeSet::new();
    for entry in &self.baselines {
      validate_name("baseline profile", &entry.profile)?;
      validate_name("baseline scenario", &entry.scenario)?;
      validate_name("baseline checkpoint", &entry.checkpoint)?;
      validate_sha256("baseline sha256", &entry.sha256)?;
      validate_sha256("baseline source", &entry.source)?;
      ensure!(
        entry.width > 0 && entry.height > 0,
        "baseline dimensions must be positive"
      );
      ensure!(entry.size_bytes > 0, "baseline size_bytes must be positive");
      ensure!(
        identities.insert((&entry.profile, &entry.scenario, &entry.checkpoint)),
        "duplicate baseline identity"
      );
    }
    Ok(())
  }
}

impl ManifestSnapshot {
  /// Reads a lock, preserving absence as a distinct state.
  pub fn read(path: &Path) -> Result<Self> {
    match fs::read(path) {
      Ok(bytes) => Ok(Self {
        manifest: Some(BaselineManifest::parse(&bytes)?),
        sha256: Some(digest(&bytes)),
      }),
      Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self {
        manifest: None,
        sha256: None,
      }),
      Err(error) => Err(error).with_context(|| format!("read {}", path.display())),
    }
  }
}

pub(crate) fn digest(bytes: &[u8]) -> String {
  format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn validate_namespace(namespace: &str) -> Result<()> {
  ensure!(
    !namespace.is_empty(),
    "baseline namespace must not be empty"
  );
  ensure!(
    namespace.split('/').all(valid_namespace_segment),
    "baseline namespace contains an invalid path component"
  );
  Ok(())
}

pub(crate) fn validate_sha256(field: &str, value: &str) -> Result<()> {
  ensure!(
    value.len() == 64
      && value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
    "{field} must be 64 lowercase hexadecimal digits"
  );
  Ok(())
}

fn validate_name(field: &str, value: &str) -> Result<()> {
  ensure!(!value.trim().is_empty(), "{field} must not be empty");
  Ok(())
}

fn valid_namespace_segment(part: &str) -> bool {
  if part.is_empty() || part == "." || part == ".." {
    return false;
  }
  part
    .bytes()
    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}
