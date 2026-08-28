//! Live review events and atomic acceptance requests.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::wire::{result::RunResult, review_validation};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvent {
  pub id: u64,
  pub body: ReviewEventBody,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "kebab-case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum ReviewEventBody {
  Snapshot {
    result: RunResult,
  },
  LogBatch {
    player_session_id: String,
    first_sequence: u64,
    last_sequence: u64,
  },
  ScenarioCompleted {
    scenario_id: String,
  },
  RunCompleted {
    run_id: String,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAcceptance {
  pub request_id: String,
  pub run_id: String,
  pub lock_sha256: Option<String>,
  pub selections: Vec<ReviewSelection>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSelection {
  pub profile: String,
  pub scenario: String,
  pub checkpoint: String,
  pub width: u32,
  pub height: u32,
  pub actual_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewAcceptanceResult {
  pub comparison_run_id: String,
  pub lock_sha256: String,
}

impl ReviewEvent {
  /// Validates the event payload and its local ordering fields.
  pub fn validate(&self) -> Result<()> {
    review_validation::validate_event(self)
  }
}

impl ReviewAcceptance {
  /// Validates an atomic request against the immutable reviewed result.
  pub fn validate(&self, reviewed: &RunResult) -> Result<()> {
    review_validation::validate_acceptance(self, reviewed)
  }
}

impl ReviewAcceptanceResult {
  /// Validates the derived run and rewritten lock identities.
  pub fn validate(&self) -> Result<()> {
    review_validation::validate_acceptance_result(self)
  }
}
