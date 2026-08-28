use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::wire::{
  baseline_state::{BaselineStoreState, BaselineTombstone},
  result_format, validation,
};

pub(super) fn validate_state(
  state: &BaselineStoreState,
  previous: Option<&BaselineStoreState>,
) -> Result<()> {
  validate_shape(state)?;
  match previous {
    Some(previous) => {
      validate_shape(previous)?;
      ensure!(
        state.generation == previous.generation.checked_add(1).unwrap_or(0),
        "baseline state generation must increment exactly once"
      );
      ensure!(
        state.published_at >= previous.published_at,
        "publication time moved backwards"
      );
    }
    None => ensure!(
      state.generation == 1,
      "initial baseline state generation must be 1"
    ),
  }
  Ok(())
}

pub(super) fn validate_shape(state: &BaselineStoreState) -> Result<()> {
  ensure!(
    state.generation > 0,
    "baseline state generation must be positive"
  );
  validation::sha256("lock_sha256", &state.lock_sha256)?;
  result_format::timestamp("published_at", &state.published_at)?;
  ensure!(
    state.live_sha256.windows(2).all(|pair| pair[0] < pair[1]),
    "live hashes must be unique and sorted"
  );
  for sha256 in &state.live_sha256 {
    validation::sha256("live_sha256", sha256)?;
  }
  validate_tombstones(&state.tombstones, &state.published_at)?;
  let live: BTreeSet<&str> = state.live_sha256.iter().map(String::as_str).collect();
  ensure!(
    state
      .tombstones
      .iter()
      .all(|tombstone| !live.contains(tombstone.sha256.as_str())),
    "live hashes and tombstones must be disjoint"
  );
  if let Some(cleanup_applied_at) = &state.cleanup_applied_at {
    result_format::timestamp("cleanup_applied_at", cleanup_applied_at)?;
  }
  Ok(())
}

fn validate_tombstones(tombstones: &[BaselineTombstone], published_at: &str) -> Result<()> {
  ensure!(
    tombstones
      .windows(2)
      .all(|pair| pair[0].sha256 < pair[1].sha256),
    "tombstones must be unique and sorted by hash"
  );
  for tombstone in tombstones {
    validation::sha256("tombstone.sha256", &tombstone.sha256)?;
    result_format::timestamp("tombstone.removed_at", &tombstone.removed_at)?;
    ensure!(
      tombstone.removed_at.as_str() <= published_at,
      "tombstone removal follows publication time"
    );
  }
  Ok(())
}
