//! Run-local error allocation, phase recording, and terminal status reduction.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use crate::wire::{
  common::{DeadlineKind, ErrorCode, ErrorSource},
  player_errors::PlayerErrorObservation,
  result::{ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus, RunStatus},
  validation,
};

/// Ownership fields attached to one run-local error occurrence.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ErrorContext {
  pub job_id: Option<String>,
  pub player_session_id: Option<String>,
  pub scenario_id: Option<String>,
  pub step_index: Option<u32>,
  pub log_sequence: Option<u64>,
}

/// How one occurrence contributes to the terminal status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureImpact {
  Functional,
  Infrastructure,
  SecondaryInfrastructure,
}

/// Accumulates ordered phases and deduplicated run-local occurrences.
#[derive(Debug, Default)]
pub struct RunOutcome {
  errors: Vec<ErrorOccurrence>,
  phases: Vec<PhaseResult>,
  identities: BTreeMap<ErrorIdentity, RecordedError>,
  primary_error_id: Option<String>,
  functional_failure: bool,
  infrastructure_failure: bool,
  interrupted: bool,
}

impl RunOutcome {
  /// Records a host-originated occurrence idempotently.
  pub fn record_host(
    &mut self,
    idempotency_key: &str,
    draft: ErrorDraft,
    impact: FailureImpact,
  ) -> Result<String> {
    ensure!(
      !idempotency_key.is_empty(),
      "host error idempotency key must not be empty"
    );
    self.record(
      ErrorIdentity::Host(idempotency_key.to_owned()),
      draft,
      impact,
    )
  }

  /// Maps one scenario-local player reference to a run-local occurrence.
  pub fn record_player(
    &mut self,
    scenario_id: &str,
    error_ref: &str,
    observation: &PlayerErrorObservation,
    mut context: ErrorContext,
    impact: FailureImpact,
  ) -> Result<String> {
    validation::identifier("scenario_id", scenario_id)?;
    player_error_ref(error_ref)?;
    ensure!(
      context
        .scenario_id
        .as_deref()
        .is_none_or(|value| value == scenario_id),
      "player error context belongs to another scenario"
    );
    ensure!(
      context
        .log_sequence
        .is_none_or(|value| Some(value) == observation.record_sequence),
      "player error context has a different log sequence"
    );
    context.scenario_id = Some(scenario_id.to_owned());
    context.log_sequence = observation.record_sequence;
    self.record(
      ErrorIdentity::Player {
        scenario_id: scenario_id.to_owned(),
        error_ref: error_ref.to_owned(),
      },
      ErrorDraft {
        code: observation.code,
        source: observation.source,
        message: observation.message.clone(),
        context,
      },
      impact,
    )
  }

  /// Records one completed phase after all referenced occurrences are allocated.
  pub fn record_phase(&mut self, phase: PhaseResult) -> Result<()> {
    validate_phase(&phase, &self.errors)?;
    self.phases.push(phase);
    Ok(())
  }

  /// Gives user interruption precedence without replacing an earlier primary error.
  pub fn mark_interrupted(&mut self) {
    self.interrupted = true;
  }

  /// Returns occurrences in allocation order.
  pub fn errors(&self) -> &[ErrorOccurrence] {
    &self.errors
  }

  /// Returns phases in execution order.
  pub fn phases(&self) -> &[PhaseResult] {
    &self.phases
  }

  /// Returns the first primary functional or infrastructure occurrence.
  pub fn primary_error_id(&self) -> Option<&str> {
    self.primary_error_id.as_deref()
  }

  /// Reduces all observed outcomes using interrupt and infrastructure precedence.
  pub fn status(&self) -> RunStatus {
    if self.interrupted {
      RunStatus::Interrupted
    } else if self.infrastructure_failure {
      RunStatus::InfrastructureError
    } else if self.functional_failure {
      RunStatus::Failed
    } else {
      RunStatus::Passed
    }
  }

  /// Returns the stable process exit code for the reduced status.
  pub fn exit_code(&self) -> u8 {
    match self.status() {
      RunStatus::Passed => 0,
      RunStatus::Failed => 1,
      RunStatus::InfrastructureError => 2,
      RunStatus::Interrupted => 130,
    }
  }

  fn record(
    &mut self,
    identity: ErrorIdentity,
    draft: ErrorDraft,
    impact: FailureImpact,
  ) -> Result<String> {
    validate_draft(&draft)?;
    if let Some(existing) = self.identities.get(&identity) {
      ensure!(
        existing.draft == draft,
        "error replay changed occurrence data"
      );
      ensure!(
        existing.impact == impact,
        "error replay changed failure impact"
      );
      return Ok(existing.error_id.clone());
    }
    let next = self
      .errors
      .len()
      .checked_add(1)
      .filter(|next| *next <= 9999)
      .ok_or_else(|| anyhow::anyhow!("run error occurrence limit exceeded"))?;
    let error_id = format!("E{next:04}");
    self.errors.push(draft.to_occurrence(error_id.clone()));
    self.identities.insert(
      identity,
      RecordedError {
        error_id: error_id.clone(),
        draft,
        impact,
      },
    );
    self.apply_impact(&error_id, impact);
    Ok(error_id)
  }

  fn apply_impact(&mut self, error_id: &str, impact: FailureImpact) {
    match impact {
      FailureImpact::Functional => self.functional_failure = true,
      FailureImpact::Infrastructure | FailureImpact::SecondaryInfrastructure => {
        self.infrastructure_failure = true;
      }
    }
    if impact != FailureImpact::SecondaryInfrastructure && self.primary_error_id.is_none() {
      self.primary_error_id = Some(error_id.to_owned());
    }
  }
}

/// Stable error data before allocation of its run-local ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorDraft {
  pub code: ErrorCode,
  pub source: ErrorSource,
  pub message: String,
  pub context: ErrorContext,
}

impl ErrorDraft {
  fn to_occurrence(&self, id: String) -> ErrorOccurrence {
    ErrorOccurrence {
      id,
      code: self.code,
      source: self.source,
      message: self.message.clone(),
      job_id: self.context.job_id.clone(),
      player_session_id: self.context.player_session_id.clone(),
      scenario_id: self.context.scenario_id.clone(),
      step_index: self.context.step_index,
      log_sequence: self.context.log_sequence,
    }
  }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ErrorIdentity {
  Host(String),
  Player {
    scenario_id: String,
    error_ref: String,
  },
}

#[derive(Debug)]
struct RecordedError {
  error_id: String,
  draft: ErrorDraft,
  impact: FailureImpact,
}

fn validate_draft(draft: &ErrorDraft) -> Result<()> {
  ensure!(!draft.message.is_empty(), "error message must not be empty");
  ensure!(draft.message.len() <= 4096, "error message is too long");
  validate_context(&draft.context)
}

fn validate_context(context: &ErrorContext) -> Result<()> {
  for (field, value) in [
    ("job_id", context.job_id.as_deref()),
    ("player_session_id", context.player_session_id.as_deref()),
    ("scenario_id", context.scenario_id.as_deref()),
  ] {
    if let Some(value) = value {
      validation::identifier(field, value)?;
    }
  }
  ensure!(
    context.step_index.is_none() || context.scenario_id.is_some(),
    "step_index requires scenario_id"
  );
  if context.log_sequence.is_some() {
    ensure!(context.job_id.is_some(), "log_sequence requires job_id");
    ensure!(
      context.player_session_id.is_some(),
      "log_sequence requires player_session_id"
    );
  }
  Ok(())
}

fn validate_phase(phase: &PhaseResult, errors: &[ErrorOccurrence]) -> Result<()> {
  let known: BTreeSet<&str> = errors.iter().map(|error| error.id.as_str()).collect();
  let mut unique = BTreeSet::new();
  for error_id in &phase.error_ids {
    ensure!(
      known.contains(error_id.as_str()),
      "phase references an unknown error"
    );
    ensure!(unique.insert(error_id), "phase error IDs must be unique");
  }
  if phase.status == PhaseStatus::Passed {
    ensure!(
      phase.error_ids.is_empty(),
      "passed phase must not contain errors"
    );
    ensure!(
      phase.expired_deadline.is_none(),
      "passed phase must not expire a deadline"
    );
  }
  if let Some(deadline) = phase.expired_deadline {
    ensure!(
      deadline_matches(phase.name, deadline),
      "phase contains the wrong deadline kind"
    );
  }
  Ok(())
}

fn deadline_matches(phase: PhaseName, deadline: DeadlineKind) -> bool {
  match phase {
    PhaseName::Build => deadline == DeadlineKind::Build,
    PhaseName::Launch => deadline == DeadlineKind::Launch,
    PhaseName::Startup => deadline == DeadlineKind::Startup,
    PhaseName::SimulatorBoot => deadline == DeadlineKind::SimulatorBoot,
    PhaseName::Reset => deadline == DeadlineKind::Reset,
    PhaseName::BaselineDownload => deadline == DeadlineKind::BaselineDownload,
    PhaseName::Comparison => deadline == DeadlineKind::Comparison,
    PhaseName::Media => deadline == DeadlineKind::Media,
    PhaseName::Durability => deadline == DeadlineKind::Durability,
    PhaseName::Scenarios => matches!(
      deadline,
      DeadlineKind::Step | DeadlineKind::Scenario | DeadlineKind::Run
    ),
    PhaseName::Discovery | PhaseName::Cleanup => deadline == DeadlineKind::Run,
  }
}

fn player_error_ref(value: &str) -> Result<()> {
  ensure!(
    value.len() == 5 && value.starts_with('P'),
    "player error reference must use P####"
  );
  ensure!(
    value[1..].bytes().all(|byte| byte.is_ascii_digit()),
    "player error reference must use P####"
  );
  ensure!(value != "P0000", "player error reference must be positive");
  Ok(())
}
