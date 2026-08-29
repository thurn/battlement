//! Canonical published state for a baseline namespace.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::wire::{baseline_state_validation, result_format};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineStoreState {
  pub generation: u64,
  pub lock_sha256: String,
  pub published_at: String,
  pub live_sha256: Vec<String>,
  pub tombstones: Vec<BaselineTombstone>,
  pub cleanup_applied_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineTombstone {
  pub sha256: String,
  pub removed_at: String,
}

impl BaselineStoreState {
  /// Validates hashes, timestamps, ordering, and disjoint live and retained sets.
  pub fn validate_shape(&self) -> Result<()> {
    baseline_state_validation::validate_shape(self)
  }

  /// Validates hashes, timestamps, ordering, and generation semantics.
  pub fn validate(&self, previous: Option<&Self>) -> Result<()> {
    baseline_state_validation::validate_state(self, previous)
  }

  /// Serializes state with lexical keys, two-space indentation, and one newline.
  pub fn to_canonical_json(&self) -> Result<Vec<u8>> {
    baseline_state_validation::validate_shape(self)?;
    result_format::canonical_pretty_json(self)
  }
}
