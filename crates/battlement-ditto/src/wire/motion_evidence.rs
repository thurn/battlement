//! Already-sampled Motion state attached to one rendered screenshot.

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::wire::{job::Motion, lifecycle::RenderCommit, validation};

/// Logical elapsed times use exact 100 ns ticks; timeline samples use microseconds.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MotionEvidence {
  pub mode: Motion,
  pub elapsed_ticks: u64,
  pub scenario_elapsed_ticks: u64,
  pub finite_timeline_count: u32,
  pub infinite_timeline_count: u32,
  pub held_timeline_count: u32,
  pub has_pending_work: bool,
  pub has_deferred_ui_work: bool,
  pub timelines: TimelineObservations,
}

/// Bounded samples of authored slots, value playbacks and graph time sources.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineObservations {
  pub sample_count: u32,
  pub samples: Vec<TimelineObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineObservation {
  pub kind: TimelineKind,
  pub owner_id: String,
  pub slot: Option<u64>,
  pub clock: SampleClock,
  pub clock_id: Option<String>,
  pub sampled_clock_micros: Option<u64>,
  pub elapsed_micros: u64,
  pub anchor_micros: Option<u64>,
  pub held: bool,
  pub infinite: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimelineKind {
  Slot,
  ValuePlayback,
  GraphTime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SampleClock {
  Unscaled,
  Scaled,
  Controlled,
  Audio,
}

pub(crate) fn validate_render_commit(commit: &RenderCommit) -> Result<()> {
  ensure!(
    commit.frame > 0 && commit.render_generation > 0,
    "render-commit identity must be positive"
  );
  let evidence = &commit.motion;
  ensure!(
    evidence.scenario_elapsed_ticks <= evidence.elapsed_ticks,
    "invalid scenario Motion elapsed time"
  );
  ensure!(
    evidence.timelines.samples.len() == evidence.timelines.sample_count.min(16) as usize,
    "sampled timeline evidence has an invalid bound"
  );
  for sample in &evidence.timelines.samples {
    validation::identifier("Motion sample owner", &sample.owner_id)?;
    let identified = matches!(sample.clock, SampleClock::Audio | SampleClock::Controlled);
    ensure!(
      identified == sample.clock_id.is_some(),
      "Motion clock identity mismatch"
    );
    if let Some(id) = &sample.clock_id {
      validation::identifier("Motion clock ID", id)?;
    }
    ensure!(
      (sample.kind == TimelineKind::Slot) == sample.slot.is_some(),
      "Motion slot identity mismatch"
    );
  }
  Ok(())
}
