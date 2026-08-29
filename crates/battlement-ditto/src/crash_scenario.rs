use anyhow::{Context, Result, ensure};

use crate::{
  crash_reconstruction::{self, LatestScenario},
  player_supervision::PlayerExitContext,
  wire::{
    common::{StepName, StepStatus},
    job::{StepKind, VideoStep},
    lifecycle::{DittoContext, DittoEventRecord, PlayerStepResult},
    result::{
      MediaCapture, Recovery, ScenarioResult, ScenarioStatus, ScenarioTimings, ScreenshotResult,
      StepResult, VideoResult,
    },
  },
};

pub(crate) fn incomplete(
  context: &PlayerExitContext,
  latest: &LatestScenario<'_>,
  error_id: &str,
  last_sequence: Option<u64>,
) -> Result<ScenarioResult> {
  let mut reached_active = false;
  let steps = latest
    .scenario
    .steps
    .iter()
    .map(|expected| {
      if let Some(player) = step_ended(
        &context.durable.records,
        &latest.scenario.id,
        expected.index,
      ) {
        return completed_step(context, &expected.action, player, error_id);
      }
      if latest.active_step == Some(expected.index) {
        reached_active = true;
        return Ok(crashed_step(
          expected.index,
          expected.name.clone(),
          &expected.action,
          error_id,
        ));
      }
      Ok(not_run_step(
        expected.index,
        expected.name.clone(),
        &expected.action,
      ))
    })
    .collect::<Result<Vec<_>>>()?;
  ensure!(
    reached_active || steps.iter().all(|step| step.status == StepStatus::NotRun),
    "durable step context has an invalid open range"
  );
  Ok(ScenarioResult {
    id: latest.scenario.id.clone(),
    name: latest.scenario.name.clone(),
    status: ScenarioStatus::Failed,
    status_reason: None,
    motion: latest.scenario.motion,
    duration_ms: steps.iter().map(|step| step.duration_ms).sum(),
    expired_deadline: None,
    timings: ScenarioTimings::default(),
    steps,
    logs: crash_reconstruction::log_span(context, latest.first_sequence, last_sequence, false),
    failure_frame: Some(MediaCapture::Unavailable {
      reason: "player exited before failure capture completed".to_owned(),
      error_id: Some(error_id.to_owned()),
    }),
    recovery: Recovery::Relaunch,
  })
}

fn completed_step(
  context: &PlayerExitContext,
  action: &StepKind,
  player: PlayerStepResult,
  exit_error_id: &str,
) -> Result<StepResult> {
  let error_ids = player
    .error_refs
    .iter()
    .map(|error_ref| {
      context
        .player_error_ids
        .get(error_ref)
        .cloned()
        .with_context(|| format!("missing host error for {error_ref}"))
    })
    .collect::<Result<Vec<_>>>()?;
  let screenshot =
    matches!(action, StepKind::Screenshot(_)).then(|| ScreenshotResult::Unavailable {
      reason: "player exited before host image processing".to_owned(),
      error_id: exit_error_id.to_owned(),
    });
  let video =
    matches!(action, StepKind::Video(VideoStep::Start { .. })).then(|| VideoResult::Failed {
      error_id: exit_error_id.to_owned(),
      diagnostic_paths: Vec::new(),
    });
  Ok(StepResult {
    index: player.index,
    name: player.name,
    kind: player.kind,
    status: player.status,
    status_reason: None,
    duration_ms: player.duration_ms,
    expired_deadline: player.expired_deadline,
    error_ids,
    assertion: player.assertion,
    screenshot,
    video,
  })
}

fn crashed_step(index: u32, name: Option<String>, action: &StepKind, error_id: &str) -> StepResult {
  StepResult {
    index,
    name,
    kind: step_kind(action),
    status: StepStatus::InfrastructureError,
    status_reason: None,
    duration_ms: 0,
    expired_deadline: None,
    error_ids: vec![error_id.to_owned()],
    assertion: None,
    screenshot: matches!(action, StepKind::Screenshot(_)).then(|| ScreenshotResult::Unavailable {
      reason: "player exited during screenshot capture".to_owned(),
      error_id: error_id.to_owned(),
    }),
    video: matches!(action, StepKind::Video(VideoStep::Start { .. })).then(|| {
      VideoResult::Failed {
        error_id: error_id.to_owned(),
        diagnostic_paths: Vec::new(),
      }
    }),
  }
}

fn not_run_step(index: u32, name: Option<String>, action: &StepKind) -> StepResult {
  StepResult {
    index,
    name,
    kind: step_kind(action),
    status: StepStatus::NotRun,
    status_reason: Some("player-exited".to_owned()),
    duration_ms: 0,
    expired_deadline: None,
    error_ids: Vec::new(),
    assertion: None,
    screenshot: None,
    video: None,
  }
}

pub(crate) fn step_ended(
  records: &[DittoEventRecord],
  scenario_id: &str,
  index: u32,
) -> Option<PlayerStepResult> {
  records.iter().find_map(|record| match record {
    DittoEventRecord::Context(record) => match &record.body {
      DittoContext::StepEnded {
        scenario_id: owner,
        result,
      } if owner == scenario_id && result.index == index => Some(result.clone()),
      _ => None,
    },
    DittoEventRecord::Log(_) => None,
  })
}

pub(crate) fn step_kind(kind: &StepKind) -> StepName {
  match kind {
    StepKind::Click { .. } => StepName::Click,
    StepKind::Hover { .. } => StepName::Hover,
    StepKind::Drag { .. } => StepName::Drag,
    StepKind::Key { .. } => StepName::Key,
    StepKind::Wait(_) => StepName::Wait,
    StepKind::Assert(_) => StepName::Assert,
    StepKind::Screenshot(_) => StepName::Screenshot,
    StepKind::Video(_) => StepName::Video,
  }
}
