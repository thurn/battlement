use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::wire::{
  result::{RunResult, ScreenshotResult},
  review::{
    ReviewAcceptance, ReviewAcceptanceResult, ReviewEvent, ReviewEventBody, ReviewSelection,
  },
  validation,
};

pub(super) fn validate_event(event: &ReviewEvent) -> Result<()> {
  ensure!(event.id > 0, "review event id must be positive");
  match &event.body {
    ReviewEventBody::Snapshot { result } => result.validate(),
    ReviewEventBody::LogBatch {
      player_session_id,
      first_sequence,
      last_sequence,
    } => {
      validation::identifier("player_session_id", player_session_id)?;
      ensure!(
        first_sequence <= last_sequence,
        "review log batch range is reversed"
      );
      Ok(())
    }
    ReviewEventBody::ScenarioCompleted { scenario_id } => {
      validation::identifier("scenario_id", scenario_id)
    }
    ReviewEventBody::RunCompleted { run_id } => validation::identifier("run_id", run_id),
  }
}

pub(super) fn validate_acceptance(
  acceptance: &ReviewAcceptance,
  reviewed: &RunResult,
) -> Result<()> {
  reviewed.validate()?;
  validation::identifier("request_id", &acceptance.request_id)?;
  validation::identifier("run_id", &acceptance.run_id)?;
  ensure!(
    acceptance.run_id == reviewed.run_id,
    "acceptance identifies another run"
  );
  ensure!(
    acceptance.lock_sha256 == reviewed.lock_sha256,
    "acceptance lock digest is stale"
  );
  if let Some(lock_sha256) = &acceptance.lock_sha256 {
    validation::sha256("lock_sha256", lock_sha256)?;
  }
  ensure!(
    !acceptance.selections.is_empty(),
    "acceptance requires at least one selection"
  );
  let mut identities = BTreeSet::new();
  for selection in &acceptance.selections {
    validation::name("selection.profile", &selection.profile)?;
    validation::name("selection.scenario", &selection.scenario)?;
    validation::name("selection.checkpoint", &selection.checkpoint)?;
    validation::sha256("selection.actual_sha256", &selection.actual_sha256)?;
    ensure!(
      selection.width > 0 && selection.height > 0,
      "selection dimensions must be positive"
    );
    ensure!(
      identities.insert((
        &selection.profile,
        &selection.scenario,
        &selection.checkpoint
      )),
      "acceptance selections must be duplicate-free"
    );
    ensure!(
      reviewed.profile.as_ref() == Some(&selection.profile),
      "selection profile does not match run"
    );
    ensure!(
      selection_matches(reviewed, selection),
      "selection does not match a captured actual image"
    );
  }
  Ok(())
}

pub(super) fn validate_acceptance_result(result: &ReviewAcceptanceResult) -> Result<()> {
  validation::identifier("comparison_run_id", &result.comparison_run_id)?;
  validation::sha256("lock_sha256", &result.lock_sha256)
}

fn selection_matches(reviewed: &RunResult, selection: &ReviewSelection) -> bool {
  reviewed.scenarios.iter().any(|scenario| {
    if scenario.name != selection.scenario {
      return false;
    }
    scenario.steps.iter().any(|step| match &step.screenshot {
      Some(ScreenshotResult::Captured {
        checkpoint, actual, ..
      }) => [
        checkpoint == &selection.checkpoint,
        actual.width == selection.width,
        actual.height == selection.height,
        actual.sha256 == selection.actual_sha256,
      ]
      .into_iter()
      .all(|matches| matches),
      _ => false,
    })
  })
}
