use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::wire::{
  common::StepStatus,
  job::{Job, ResolvedScenario, ResolvedStep, StepKind, VideoStep},
  lifecycle::{
    ArtifactKind, ExecutionStatus, PlayerFailureFrame, PlayerStepResult, ScenarioBoundaryOutcome,
    ScenarioComplete,
  },
  lifecycle_validation, validation,
};

pub(super) fn scenario_complete(
  complete: &ScenarioComplete,
  job: &Job,
  observed_error_refs: &[String],
) -> Result<()> {
  let scenario = lifecycle_validation::scenario(job, &complete.scenario_id)?;
  ensure!(
    complete.steps.len() == scenario.steps.len(),
    "completion must retain every authored step"
  );
  ensure!(
    complete.artifacts.len() <= 128,
    "scenario may reach at most 128 artifacts"
  );
  ensure!(
    complete.video_inputs.len() <= 64,
    "scenario may contain at most 64 video inputs"
  );
  let elapsed = complete
    .startup_duration_ms
    .checked_add(complete.execution_duration_ms)
    .ok_or_else(|| anyhow::anyhow!("scenario duration overflow"))?;
  ensure!(
    elapsed <= scenario.timeout_ms,
    "scenario completion exceeds its deadline"
  );
  let observed: BTreeSet<&String> = observed_error_refs.iter().collect();
  ensure!(
    observed.len() == observed_error_refs.len(),
    "observed error references must be unique"
  );
  for error_ref in observed_error_refs {
    lifecycle_validation::player_error_ref(error_ref)?;
  }
  for (expected, result) in scenario.steps.iter().zip(&complete.steps) {
    step_result(expected, result)?;
    for error_ref in &result.error_refs {
      ensure!(
        observed.contains(error_ref),
        "step contains an unobserved error reference"
      );
    }
  }
  validate_status(complete, &observed)?;
  let artifact_ids = artifacts(complete, scenario)?;
  failure_frame(complete, &artifact_ids, &observed)?;
  videos(complete, scenario)?;
  boundary(&complete.boundary, &observed)?;
  Ok(())
}

pub(super) fn step_result(expected: &ResolvedStep, result: &PlayerStepResult) -> Result<()> {
  ensure!(
    result.index == expected.index,
    "step result index does not match the job"
  );
  ensure!(
    result.name == expected.name,
    "step result name does not match the job"
  );
  ensure!(
    result.kind == lifecycle_validation::step_name(&expected.action),
    "step result kind does not match the job"
  );
  ensure!(
    result.duration_ms <= expected.timeout_ms,
    "step result exceeds its deadline"
  );
  ensure!(
    result.error_refs.len() <= 16,
    "step may contain at most 16 error references"
  );
  let mut unique = BTreeSet::new();
  for error_ref in &result.error_refs {
    lifecycle_validation::player_error_ref(error_ref)?;
    ensure!(
      unique.insert(error_ref),
      "step error references must be unique"
    );
  }
  match result.status {
    StepStatus::Passed => ensure!(
      result.expired_deadline.is_none() && result.error_refs.is_empty(),
      "passed step must not contain an expiry or errors"
    ),
    StepStatus::Failed | StepStatus::InfrastructureError => ensure!(
      !result.error_refs.is_empty(),
      "failed step requires an error reference"
    ),
    StepStatus::NotRun => ensure!(
      result.duration_ms == 0
        && result.expired_deadline.is_none()
        && result.error_refs.is_empty()
        && result.assertion.is_none()
        && result.screenshot_artifact_id.is_none()
        && result.video_input_id.is_none(),
      "not-run step must have zero duration and no reached payload"
    ),
    StepStatus::Interrupted => {}
  }
  assertion(expected, result)?;
  screenshot(expected, result)?;
  video(expected, result)
}

fn assertion(expected: &ResolvedStep, result: &PlayerStepResult) -> Result<()> {
  let Some(assertion) = &result.assertion else {
    ensure!(
      !matches!(expected.action, StepKind::Assert(_))
        || !matches!(result.status, StepStatus::Passed | StepStatus::Failed),
      "completed assertion step requires an assertion payload"
    );
    return Ok(());
  };
  let StepKind::Assert(condition) = &expected.action else {
    anyhow::bail!("assertion payload belongs only to an assertion step");
  };
  validation::identifier("assertion object", &assertion.object)?;
  ensure!(
    assertion.object == condition.object,
    "assertion object does not match the job"
  );
  ensure!(
    assertion.state == condition.state,
    "assertion state does not match the job"
  );
  ensure!(assertion.expected, "assertion expected value must be true");
  ensure!(
    assertion.passed == assertion.observed,
    "assertion passed must equal observed"
  );
  ensure!(
    assertion.passed == (result.status == StepStatus::Passed),
    "assertion result must agree with step status"
  );
  Ok(())
}

fn screenshot(expected: &ResolvedStep, result: &PlayerStepResult) -> Result<()> {
  let Some(artifact_id) = &result.screenshot_artifact_id else {
    ensure!(
      !matches!(expected.action, StepKind::Screenshot(_)) || result.status != StepStatus::Passed,
      "passed screenshot step requires an artifact ID"
    );
    return Ok(());
  };
  ensure!(
    matches!(expected.action, StepKind::Screenshot(_)),
    "screenshot artifact belongs only to a screenshot step"
  );
  validation::identifier("screenshot artifact_id", artifact_id)
}

fn video(expected: &ResolvedStep, result: &PlayerStepResult) -> Result<()> {
  let Some(input_id) = &result.video_input_id else {
    return Ok(());
  };
  ensure!(
    matches!(expected.action, StepKind::Video(VideoStep::Start { .. })),
    "video input belongs only to a video start step"
  );
  validation::identifier("video input_id", input_id)
}

fn validate_status(complete: &ScenarioComplete, observed: &BTreeSet<&String>) -> Result<()> {
  if let Some(primary) = &complete.primary_error_ref {
    lifecycle_validation::player_error_ref(primary)?;
    ensure!(
      observed.contains(primary),
      "primary_error_ref must resolve to an observed error"
    );
  }
  match complete.execution_status {
    ExecutionStatus::Passed => {
      ensure!(
        complete.primary_error_ref.is_none(),
        "passed scenario has no primary error"
      );
      ensure!(
        complete
          .steps
          .iter()
          .all(|step| step.status == StepStatus::Passed),
        "passed scenario requires every step to pass"
      );
    }
    ExecutionStatus::Failed => ensure!(
      complete.primary_error_ref.is_some(),
      "failed scenario requires a primary error reference"
    ),
    ExecutionStatus::Interrupted => {}
  }
  Ok(())
}

fn artifacts<'a>(
  complete: &'a ScenarioComplete,
  scenario: &ResolvedScenario,
) -> Result<BTreeSet<&'a String>> {
  let mut ids = BTreeSet::new();
  for artifact in &complete.artifacts {
    validation::identifier("artifact_id", &artifact.artifact_id)?;
    ensure!(
      ids.insert(&artifact.artifact_id),
      "reached artifact IDs must be unique"
    );
    lifecycle_validation::artifact_kind(&artifact.kind)?;
    if let Some(index) = artifact.step_index {
      ensure!(
        (index as usize) < scenario.steps.len(),
        "artifact step_index is outside the scenario"
      );
    }
    match &artifact.kind {
      ArtifactKind::Screenshot { checkpoint } => {
        let Some(index) = artifact.step_index else {
          anyhow::bail!("screenshot artifact requires a step_index");
        };
        let StepKind::Screenshot(expected) = &scenario.steps[index as usize].action else {
          anyhow::bail!("screenshot artifact must reference a screenshot step");
        };
        ensure!(
          checkpoint == &expected.name,
          "artifact checkpoint does not match the job"
        );
        ensure!(
          complete.steps[index as usize]
            .screenshot_artifact_id
            .as_ref()
            == Some(&artifact.artifact_id),
          "artifact ID does not match its screenshot step result"
        );
      }
      ArtifactKind::FailureFrame => {
        let Some(PlayerFailureFrame::Captured { artifact_id }) = &complete.failure_frame else {
          anyhow::bail!("failure-frame artifact requires a captured failure frame");
        };
        ensure!(
          artifact_id == &artifact.artifact_id,
          "failure-frame artifact does not match the captured frame"
        );
      }
    }
  }
  for step in &complete.steps {
    if let Some(artifact_id) = &step.screenshot_artifact_id {
      ensure!(
        ids.contains(artifact_id),
        "step references an unknown artifact"
      );
    }
  }
  Ok(ids)
}

fn failure_frame(
  complete: &ScenarioComplete,
  artifact_ids: &BTreeSet<&String>,
  observed: &BTreeSet<&String>,
) -> Result<()> {
  let Some(frame) = &complete.failure_frame else {
    return Ok(());
  };
  lifecycle_validation::failure_frame(frame)?;
  match frame {
    PlayerFailureFrame::Captured { artifact_id } => {
      ensure!(
        artifact_ids.contains(artifact_id),
        "failure frame references an unknown artifact"
      );
      ensure!(
        complete.artifacts.iter().any(|artifact| {
          artifact.artifact_id == *artifact_id && artifact.kind == ArtifactKind::FailureFrame
        }),
        "captured failure frame must reference a failure-frame artifact"
      );
    }
    PlayerFailureFrame::Unavailable {
      error_ref: Some(error_ref),
      ..
    } => ensure!(
      observed.contains(error_ref),
      "failure frame contains an unobserved error reference"
    ),
    PlayerFailureFrame::Unavailable {
      error_ref: None, ..
    } => {}
  }
  Ok(())
}

fn videos(complete: &ScenarioComplete, scenario: &ResolvedScenario) -> Result<()> {
  let mut ids = BTreeSet::new();
  for input in &complete.video_inputs {
    lifecycle_validation::native_video(input, scenario)?;
    ensure!(
      ids.insert(&input.input_id),
      "native video input IDs must be unique"
    );
    ensure!(
      complete.steps[input.start_step_index as usize]
        .video_input_id
        .as_ref()
        == Some(&input.input_id),
      "native video input does not match its start-step result"
    );
  }
  for step in &complete.steps {
    if let Some(input_id) = &step.video_input_id {
      ensure!(
        ids.contains(input_id),
        "step references an unknown native video input"
      );
    }
  }
  Ok(())
}

fn boundary(boundary: &ScenarioBoundaryOutcome, observed: &BTreeSet<&String>) -> Result<()> {
  if let ScenarioBoundaryOutcome::Failed { error_ref, .. } = boundary {
    lifecycle_validation::player_error_ref(error_ref)?;
    ensure!(
      observed.contains(error_ref),
      "boundary contains an unobserved error reference"
    );
  }
  Ok(())
}
