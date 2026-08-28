//! Scenario-local allocation and correlation of player failure references.

use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use crate::wire::common::{ErrorCode, ErrorSource};

/// One classified player failure before host occurrence allocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerErrorObservation {
  pub code: ErrorCode,
  pub source: ErrorSource,
  pub message: String,
  pub record_sequence: Option<u64>,
  pub battlement_error_id: Option<String>,
}

/// Allocates scenario-local `P####` references and reuses structured correlations.
#[derive(Debug, Default)]
pub struct PlayerErrorMapper {
  observations: BTreeMap<ObservationIdentity, MappedObservation>,
}

impl PlayerErrorMapper {
  /// Records one failure, returning its stable scenario-local reference.
  pub fn observe(
    &mut self,
    observation_id: &str,
    observation: PlayerErrorObservation,
  ) -> Result<String> {
    validate_observation_id(observation_id)?;
    validate_observation(&observation)?;
    let identity = ObservationIdentity::new(observation_id, &observation);
    if let Some(existing) = self.observations.get(&identity) {
      ensure!(
        existing.observation == observation,
        "player error correlation was reused with different failure data"
      );
      return Ok(existing.error_ref.clone());
    }
    let next = self
      .observations
      .len()
      .checked_add(1)
      .filter(|next| *next <= 9999)
      .ok_or_else(|| anyhow::anyhow!("player error reference limit exceeded"))?;
    let error_ref = format!("P{next:04}");
    self.observations.insert(
      identity,
      MappedObservation {
        error_ref: error_ref.clone(),
        observation,
      },
    );
    Ok(error_ref)
  }

  /// Confirms that a caught-failure envelope correlates without allocating a reference.
  pub fn suppress_caught_failure(&self, battlement_error_id: &str) -> Result<()> {
    ensure!(
      !battlement_error_id.is_empty(),
      "caught failure ID must not be empty"
    );
    ensure!(
      self
        .observations
        .contains_key(&ObservationIdentity::Battlement(
          battlement_error_id.to_owned()
        )),
      "caught failure envelope has no structured Battlement error"
    );
    Ok(())
  }

  /// Returns the classified observation for a previously allocated reference.
  pub fn observation(&self, error_ref: &str) -> Option<&PlayerErrorObservation> {
    self
      .observations
      .values()
      .find(|mapped| mapped.error_ref == error_ref)
      .map(|mapped| &mapped.observation)
  }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ObservationIdentity {
  Battlement(String),
  Record(u64),
  Local(String),
}

impl ObservationIdentity {
  fn new(observation_id: &str, observation: &PlayerErrorObservation) -> Self {
    if let Some(battlement_error_id) = &observation.battlement_error_id {
      Self::Battlement(battlement_error_id.clone())
    } else if let Some(record_sequence) = observation.record_sequence {
      Self::Record(record_sequence)
    } else {
      Self::Local(observation_id.to_owned())
    }
  }
}

#[derive(Debug)]
struct MappedObservation {
  error_ref: String,
  observation: PlayerErrorObservation,
}

fn validate_observation_id(value: &str) -> Result<()> {
  ensure!(!value.is_empty(), "player observation ID must not be empty");
  ensure!(value.len() <= 128, "player observation ID is too long");
  Ok(())
}

fn validate_observation(observation: &PlayerErrorObservation) -> Result<()> {
  ensure!(
    !observation.message.is_empty(),
    "player error message must not be empty"
  );
  ensure!(
    observation.message.len() <= 4096,
    "player error message is too long"
  );
  if let Some(error_id) = &observation.battlement_error_id {
    ensure!(
      !error_id.is_empty(),
      "Battlement error ID must not be empty"
    );
    ensure!(error_id.len() <= 128, "Battlement error ID is too long");
  }
  Ok(())
}
