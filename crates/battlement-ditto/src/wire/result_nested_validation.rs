use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use crate::wire::{
  common::{DeadlineKind, StepName, StepStatus},
  result::{
    BaselineOutcome, ComparisonOutcome, ErrorOccurrence, ImageFile, MediaCapture, ResultCommand,
    RunResult, ScenarioResult, ScenarioStatus, ScreenshotResult, StepResult, VideoResult,
  },
  result_validation, validation,
};

pub(super) fn scenario(
  scenario: &ScenarioResult,
  command: ResultCommand,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  let unstarted = matches!(
    scenario.status,
    ScenarioStatus::Skipped | ScenarioStatus::NotRun
  );
  for (index, step) in scenario.steps.iter().enumerate() {
    ensure!(
      step.index == index as u32,
      "step indices must match result order"
    );
    if let Some(name) = &step.name {
      validation::name("step.name", name)?;
    }
    ensure!(
      !unstarted || step.status == StepStatus::NotRun,
      "unstarted scenario steps must be not-run"
    );
    if unstarted {
      ensure!(
        step.status_reason == scenario.status_reason,
        "unstarted steps must carry the scenario reason"
      );
    }
    validate_step(step, command, errors, artifacts)?;
  }
  if let Some(frame) = &scenario.failure_frame {
    media_capture(frame, errors, artifacts)?;
  }
  Ok(())
}

pub(super) fn baseline_writes(result: &RunResult) -> Result<()> {
  ensure!(
    result.command != ResultCommand::Capture || result.baseline_writes.is_empty(),
    "capture may not contain baseline writes"
  );
  let mut identities = BTreeSet::new();
  for write in &result.baseline_writes {
    validation::sha256("baseline write sha256", &write.sha256)?;
    validation::name("baseline write profile", &write.profile)?;
    validation::name("baseline write scenario", &write.scenario)?;
    validation::name("baseline write checkpoint", &write.checkpoint)?;
    ensure!(
      identities.insert((&write.profile, &write.scenario, &write.checkpoint)),
      "baseline write identities must be unique"
    );
    if let Some(profile) = &result.profile {
      ensure!(
        write.profile == *profile,
        "baseline write profile does not match result"
      );
    }
    let actual_sha256 = result.scenarios.iter().find_map(|scenario| {
      if scenario.name != write.scenario {
        return None;
      }
      scenario
        .steps
        .iter()
        .find_map(|step| match &step.screenshot {
          Some(ScreenshotResult::Captured {
            checkpoint, actual, ..
          }) if checkpoint == &write.checkpoint => Some(actual.sha256.as_str()),
          _ => None,
        })
    });
    ensure!(
      actual_sha256 == Some(write.sha256.as_str()),
      "baseline write does not match a captured actual image"
    );
  }
  Ok(())
}

fn validate_step(
  step: &StepResult,
  command: ResultCommand,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  let not_run = step.status == StepStatus::NotRun;
  ensure!(
    step.status_reason.is_some() == not_run,
    "step status_reason does not match status"
  );
  if let Some(reason) = &step.status_reason {
    diagnostic_reason("step status_reason", reason)?;
  }
  result_validation::unique_error_references("step.error_ids", &step.error_ids, errors)?;
  if not_run {
    ensure!(step.duration_ms == 0, "not-run step duration must be zero");
    ensure!(
      step.expired_deadline.is_none(),
      "not-run step has no expired deadline"
    );
    ensure!(step.error_ids.is_empty(), "not-run step has no errors");
    ensure!(step.assertion.is_none(), "not-run step has no assertion");
    ensure!(step.screenshot.is_none(), "not-run step has no screenshot");
    ensure!(step.video.is_none(), "not-run step has no video");
    return Ok(());
  }
  ensure!(
    step.expired_deadline.is_none_or(|deadline| {
      matches!(
        deadline,
        DeadlineKind::Step | DeadlineKind::Scenario | DeadlineKind::Run
      )
    }),
    "step expired_deadline must be step, scenario, or run"
  );
  match step.kind {
    StepName::Assert => {
      ensure!(
        step.assertion.is_some(),
        "reached assertion requires assertion result"
      );
      ensure!(
        step.screenshot.is_none(),
        "assertion step must not contain screenshot"
      );
      ensure!(
        step.video.is_none(),
        "assertion step must not contain video"
      );
    }
    StepName::Screenshot => {
      ensure!(
        step.assertion.is_none(),
        "screenshot step must not contain assertion"
      );
      ensure!(
        step.screenshot.is_some(),
        "reached screenshot requires screenshot result"
      );
      ensure!(
        step.video.is_none(),
        "screenshot step must not contain video"
      );
    }
    StepName::Video => {
      ensure!(
        step.assertion.is_none(),
        "video step must not contain assertion"
      );
      ensure!(
        step.screenshot.is_none(),
        "video step must not contain screenshot"
      );
    }
    _ => {
      ensure!(
        step.assertion.is_none(),
        "action step must not contain assertion"
      );
      ensure!(
        step.screenshot.is_none(),
        "action step must not contain screenshot"
      );
      ensure!(step.video.is_none(), "action step must not contain video");
    }
  }
  if let Some(assertion) = &step.assertion {
    validation::identifier("assertion.object", &assertion.object)?;
    ensure!(assertion.expected, "assertion expected must be true");
    ensure!(
      assertion.passed == assertion.observed,
      "assertion passed must equal observed"
    );
  }
  if let Some(screenshot) = &step.screenshot {
    screenshot_result(screenshot, command, step.status, errors, artifacts)?;
  }
  if let Some(video) = &step.video {
    video_result(video, errors, artifacts)?;
  }
  Ok(())
}

fn screenshot_result(
  screenshot: &ScreenshotResult,
  command: ResultCommand,
  step_status: StepStatus,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  match screenshot {
    ScreenshotResult::Captured {
      checkpoint,
      actual,
      baseline,
      comparison,
      matched_before_update,
      updated,
    } => {
      validation::name("screenshot checkpoint", checkpoint)?;
      image("screenshot actual", actual, artifacts)?;
      ensure!(
        matched_before_update.is_some() == updated.is_some(),
        "screenshot update fields must be paired"
      );
      ensure!(
        command != ResultCommand::Capture || matched_before_update.is_none(),
        "capture must not contain update fields"
      );
      ensure!(
        command == ResultCommand::Run || matched_before_update.is_none(),
        "update fields are valid only for executed run updates"
      );
      update_fields(
        *matched_before_update,
        *updated,
        comparison.as_ref(),
        step_status,
      )?;
      baseline_and_comparison(actual, baseline, comparison.as_ref(), artifacts)?;
      if command == ResultCommand::Capture {
        ensure!(
          matches!(baseline, BaselineOutcome::NotLoaded),
          "capture baseline must be not-loaded"
        );
        ensure!(comparison.is_none(), "capture must not compare screenshots");
      } else {
        ensure!(
          !matches!(baseline, BaselineOutcome::NotLoaded),
          "non-capture screenshot must load or miss a baseline"
        );
      }
    }
    ScreenshotResult::Unavailable { reason, error_id } => {
      diagnostic_reason("screenshot unavailable reason", reason)?;
      result_validation::error_reference("screenshot error_id", error_id, errors)?;
    }
  }
  Ok(())
}

fn baseline_and_comparison(
  actual: &ImageFile,
  baseline: &BaselineOutcome,
  comparison: Option<&ComparisonOutcome>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  match baseline {
    BaselineOutcome::NotLoaded | BaselineOutcome::Missing => {
      ensure!(
        comparison.is_none(),
        "unloaded or missing baseline cannot be compared"
      );
    }
    BaselineOutcome::Loaded {
      image: baseline_image,
    } => {
      image("screenshot baseline", baseline_image, artifacts)?;
      ensure!(
        baseline_image.width == actual.width,
        "baseline width does not match actual"
      );
      ensure!(
        baseline_image.height == actual.height,
        "baseline height does not match actual"
      );
      ensure!(
        comparison.is_some(),
        "loaded baseline requires a comparison"
      );
    }
  }
  if let Some(comparison) = comparison {
    comparison_outcome(actual, comparison, artifacts)?;
  }
  Ok(())
}

fn update_fields(
  matched_before_update: Option<bool>,
  updated: Option<bool>,
  comparison: Option<&ComparisonOutcome>,
  step_status: StepStatus,
) -> Result<()> {
  let (Some(matched), Some(updated)) = (matched_before_update, updated) else {
    return Ok(());
  };
  ensure!(
    !matched || !updated,
    "matching screenshot must not be updated"
  );
  if matched {
    ensure!(
      matches!(comparison, Some(ComparisonOutcome::Passed { .. })),
      "matched_before_update requires a passing comparison"
    );
  }
  if updated {
    ensure!(
      !matched,
      "updated screenshot must not have matched before update"
    );
    ensure!(
      step_status == StepStatus::Passed,
      "updated screenshot step must pass"
    );
  }
  Ok(())
}

fn comparison_outcome(
  actual: &ImageFile,
  comparison: &ComparisonOutcome,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  let (changed_pixels, total_pixels, settings, diff) = match comparison {
    ComparisonOutcome::Passed {
      changed_pixels,
      total_pixels,
      settings,
    } => (*changed_pixels, *total_pixels, settings, None),
    ComparisonOutcome::Mismatch {
      changed_pixels,
      total_pixels,
      settings,
      diff,
    } => (*changed_pixels, *total_pixels, settings, Some(diff)),
  };
  let expected_pixels = u64::from(actual.width) * u64::from(actual.height);
  ensure!(
    total_pixels == expected_pixels,
    "comparison total_pixels does not match image dimensions"
  );
  ensure!(
    changed_pixels <= total_pixels,
    "comparison changed_pixels exceeds total_pixels"
  );
  validation::comparison(settings)?;
  if let Some(diff) = diff {
    ensure!(changed_pixels > 0, "mismatch must contain changed pixels");
    image("comparison diff", diff, artifacts)?;
    ensure!(
      diff.width == actual.width,
      "diff width does not match actual"
    );
    ensure!(
      diff.height == actual.height,
      "diff height does not match actual"
    );
  }
  Ok(())
}

fn video_result(
  video: &VideoResult,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  match video {
    VideoResult::Encoded {
      path,
      sha256,
      width,
      height,
      frame_rate,
      duration_ms,
      truncated: _,
    } => {
      result_validation::retained_path("encoded video path", path, artifacts)?;
      ensure!(
        path.ends_with(".mp4"),
        "encoded video path must end in .mp4"
      );
      validation::sha256("encoded video sha256", sha256)?;
      ensure!(
        *width > 0 && *height > 0,
        "encoded video dimensions must be positive"
      );
      ensure!(*frame_rate == 30, "encoded video frame rate must be 30");
      ensure!(*duration_ms > 0, "encoded video duration must be positive");
    }
    VideoResult::Failed {
      error_id,
      diagnostic_paths,
    } => {
      result_validation::error_reference("video error_id", error_id, errors)?;
      for path in diagnostic_paths {
        result_validation::retained_path("video diagnostic path", path, artifacts)?;
      }
    }
  }
  Ok(())
}

fn media_capture(
  capture: &MediaCapture,
  errors: &BTreeMap<&str, &ErrorOccurrence>,
  artifacts: &BTreeSet<&str>,
) -> Result<()> {
  match capture {
    MediaCapture::Captured { image: captured } => image("failure frame", captured, artifacts),
    MediaCapture::Unavailable { reason, error_id } => {
      diagnostic_reason("failure frame reason", reason)?;
      if let Some(error_id) = error_id {
        result_validation::error_reference("failure frame error_id", error_id, errors)?;
      }
      Ok(())
    }
  }
}

fn image(field: &str, image: &ImageFile, artifacts: &BTreeSet<&str>) -> Result<()> {
  result_validation::retained_path(field, &image.path, artifacts)?;
  ensure!(
    image.path.ends_with(".png"),
    "{field} path must end in .png"
  );
  validation::sha256(field, &image.sha256)?;
  ensure!(
    image.width > 0 && image.height > 0,
    "{field} dimensions must be positive"
  );
  Ok(())
}

fn diagnostic_reason(field: &str, value: &str) -> Result<()> {
  ensure!(!value.is_empty(), "{field} must not be empty");
  ensure!(
    value.len() <= 4096,
    "{field} may contain at most 4096 UTF-8 bytes"
  );
  Ok(())
}
