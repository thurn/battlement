//! Run-local image and scenario result helpers.

use std::{fs, path::Path};

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::wire::{
  common::{DeadlineKind, StepStatus},
  lifecycle::{
    DittoContext, DittoEventRecord, ExecutionStatus, PlayerFailureFrame, ScenarioBoundaryOutcome,
  },
  result::{
    BaselineOutcome, ImageFile, MediaCapture, ScenarioStatus, ScreenshotResult, StepResult,
  },
};

pub(crate) fn missing_screenshot(checkpoint: &str, actual: ImageFile) -> ScreenshotResult {
  ScreenshotResult::Captured {
    checkpoint: checkpoint.to_owned(),
    actual,
    baseline: BaselineOutcome::Missing,
    comparison: None,
    matched_before_update: None,
    updated: None,
  }
}

pub(crate) fn image_file(path: &Path, relative: String) -> Result<ImageFile> {
  let bytes = fs::read(path)?;
  anyhow::ensure!(
    bytes.len() >= 24 && &bytes[..8] == b"\x89PNG\r\n\x1a\n" && &bytes[12..16] == b"IHDR",
    "captured artifact is not a PNG"
  );
  Ok(ImageFile {
    path: relative,
    sha256: format!("{:x}", Sha256::digest(&bytes)),
    width: u32::from_be_bytes(bytes[16..20].try_into()?),
    height: u32::from_be_bytes(bytes[20..24].try_into()?),
  })
}

pub(crate) fn copy_artifact(source: &Path, destination: &Path) -> Result<()> {
  if let Some(parent) = destination.parent() {
    fs::create_dir_all(parent)?;
  }
  if !destination.is_file() {
    fs::copy(source, destination)?;
  }
  Ok(())
}

pub(crate) fn scenario_status(execution: ExecutionStatus, steps: &[StepResult]) -> ScenarioStatus {
  if steps
    .iter()
    .any(|step| step.status == StepStatus::InfrastructureError)
  {
    ScenarioStatus::InfrastructureError
  } else if execution == ExecutionStatus::Interrupted {
    ScenarioStatus::Interrupted
  } else if execution == ExecutionStatus::Failed
    || steps.iter().any(|step| step.status == StepStatus::Failed)
  {
    ScenarioStatus::Failed
  } else {
    ScenarioStatus::Passed
  }
}

pub(crate) fn scenario_deadline(steps: &[StepResult]) -> Option<DeadlineKind> {
  steps
    .iter()
    .find_map(|step| step.expired_deadline)
    .and_then(|deadline| {
      matches!(deadline, DeadlineKind::Scenario | DeadlineKind::Run).then_some(deadline)
    })
}

pub(crate) fn boundary_duration(boundary: &ScenarioBoundaryOutcome) -> u64 {
  match boundary {
    ScenarioBoundaryOutcome::Passed { duration_ms }
    | ScenarioBoundaryOutcome::Failed { duration_ms, .. } => *duration_ms,
  }
}

pub(crate) fn scenario_log_identity(
  records: &[DittoEventRecord],
  scenario_id: &str,
) -> Option<(u64, String)> {
  records.iter().find_map(|record| {
    let DittoEventRecord::Context(record) = record else {
      return None;
    };
    matches!(
      &record.body,
      DittoContext::ScenarioStarted { scenario_id: value } if value == scenario_id
    )
    .then(|| (record.sequence, record.player_session_id.clone()))
  })
}

pub(crate) fn failure_frame(
  run_directory: &Path,
  frame: Option<&PlayerFailureFrame>,
) -> Result<Option<MediaCapture>> {
  Ok(match frame {
    Some(PlayerFailureFrame::Captured { artifact_id }) => {
      let relative = format!("artifacts/{artifact_id}.png");
      Some(MediaCapture::Captured {
        image: image_file(&run_directory.join(&relative), relative)?,
      })
    }
    Some(PlayerFailureFrame::Unavailable {
      reason,
      error_ref: _,
    }) => Some(MediaCapture::Unavailable {
      reason: reason.clone(),
      error_id: None,
    }),
    None => None,
  })
}
