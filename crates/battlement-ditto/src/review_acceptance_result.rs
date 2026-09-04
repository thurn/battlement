use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, ensure};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
  baseline_update::{BaselineProposal, BaselineUpdateResult, ScenarioUpdate, ScenarioUpdateStatus},
  config::model::{Profile, StepKind, Suite},
  wire::{
    common::StepStatus,
    job::Comparison,
    result::{
      BaselineOutcome, BaselineWriteResult, ComparisonOutcome, ImageFile, ResultCommand, RunResult,
      RunStatus, ScenarioStatus, ScreenshotResult,
    },
    review::{ReviewAcceptance, ReviewSelection},
  },
};

pub(super) fn validate_current_suite(suite: &Suite, request: &ReviewAcceptance) -> Result<()> {
  let profile = suite
    .profiles
    .get(&request.selections[0].profile)
    .context("selection profile is absent from the current suite")?;
  for selection in &request.selections {
    ensure!(
      suite.scenarios.iter().any(|scenario| {
        scenario.name == selection.scenario
          && scenario.steps.iter().any(|step| {
            matches!(&step.action, StepKind::Screenshot(value) if value.name == selection.checkpoint)
          })
      }),
      "selection checkpoint is absent from the current full suite"
    );
    if let Profile::Macos { display } | Profile::Webgl { display, .. } = profile {
      ensure!(
        (selection.width, selection.height) == (display.width, display.height),
        "selection dimensions differ from the current profile"
      );
    }
  }
  Ok(())
}

pub(super) fn selected_actual<'a>(
  result: &'a RunResult,
  selection: &ReviewSelection,
) -> Result<&'a ImageFile> {
  result
    .scenarios
    .iter()
    .find(|scenario| scenario.name == selection.scenario)
    .and_then(|scenario| {
      scenario
        .steps
        .iter()
        .find_map(|step| match &step.screenshot {
          Some(ScreenshotResult::Captured {
            checkpoint, actual, ..
          }) if checkpoint == &selection.checkpoint => Some(actual),
          _ => None,
        })
    })
    .context("selected actual is absent from reviewed run")
}

pub(super) fn authored_checkpoints(suite: &Suite) -> BTreeMap<String, BTreeSet<String>> {
  suite
    .scenarios
    .iter()
    .map(|scenario| {
      (
        scenario.name.clone(),
        scenario
          .steps
          .iter()
          .filter_map(|step| match &step.action {
            StepKind::Screenshot(value) => Some(value.name.clone()),
            _ => None,
          })
          .collect(),
      )
    })
    .collect()
}

pub(super) fn group_proposals(proposals: Vec<BaselineProposal>) -> Vec<ScenarioUpdate> {
  proposals
    .into_iter()
    .fold(
      BTreeMap::<String, Vec<_>>::new(),
      |mut grouped, proposal| {
        grouped
          .entry(proposal.scenario.clone())
          .or_default()
          .push(proposal);
        grouped
      },
    )
    .into_iter()
    .map(|(name, proposals)| ScenarioUpdate {
      name,
      status: ScenarioUpdateStatus::Eligible,
      proposals,
    })
    .collect()
}

pub(super) fn attempt_result(source: &RunResult, now: u64) -> Result<RunResult> {
  attempt_result_with_id(source, &Uuid::new_v4().to_string(), now)
}

pub(super) fn failure_result(
  source: &RunResult,
  run_id: &str,
  writes: Vec<BaselineWriteResult>,
  now: u64,
) -> Result<RunResult> {
  let mut result = source.clone();
  set_identity(&mut result, source, run_id, now)?;
  result.baseline_writes = writes;
  normalize_capture(&mut result);
  Ok(result)
}

pub(super) fn derived_result(
  suite: &Suite,
  source: &RunResult,
  request: &ReviewAcceptance,
  applied: &BaselineUpdateResult,
  directory: &Path,
  now: u64,
) -> Result<RunResult> {
  let mut result = source.clone();
  let run_id = directory.file_name().unwrap().to_string_lossy();
  set_identity(&mut result, source, &run_id, now)?;
  result.lock_sha256 = Some(applied.lock_sha256.clone());
  result.baseline_writes = applied.writes.clone();
  for scenario in &mut result.scenarios {
    for step in &mut scenario.steps {
      let Some(ScreenshotResult::Captured {
        checkpoint,
        actual,
        baseline,
        comparison,
        matched_before_update,
        updated,
      }) = &mut step.screenshot
      else {
        continue;
      };
      *matched_before_update = None;
      *updated = None;
      let selected = request.selections.iter().any(|selection| {
        selection.scenario == scenario.name && selection.checkpoint == *checkpoint
      });
      if selected {
        let relative = format!("baselines/{}.png", actual.sha256);
        copy_baseline(&directory.join(&actual.path), &directory.join(&relative))?;
        *baseline = BaselineOutcome::Loaded {
          image: ImageFile {
            path: relative,
            ..actual.clone()
          },
        };
        *comparison = Some(ComparisonOutcome::Passed {
          changed_pixels: 0,
          total_pixels: u64::from(actual.width) * u64::from(actual.height),
          settings: comparison_settings(suite, &scenario.name, checkpoint, comparison.as_ref())?,
        });
        step.status = StepStatus::Passed;
        step.error_ids.clear();
      } else if matches!(baseline, BaselineOutcome::NotLoaded) {
        *baseline = BaselineOutcome::Missing;
        step.status = StepStatus::Failed;
      }
    }
    if scenario
      .steps
      .iter()
      .all(|step| step.status == StepStatus::Passed)
    {
      scenario.status = ScenarioStatus::Passed;
    } else if scenario.status == ScenarioStatus::Passed {
      scenario.status = ScenarioStatus::Failed;
    }
  }
  reduce_status(&mut result, source.status);
  Ok(result)
}

pub(super) fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn attempt_result_with_id(source: &RunResult, run_id: &str, now: u64) -> Result<RunResult> {
  Ok(RunResult {
    run_id: run_id.to_owned(),
    source_run_id: Some(source.run_id.clone()),
    lock_sha256: source.lock_sha256.clone(),
    command: ResultCommand::ComparisonOnly,
    source_command: source.source_command.or(Some(source.command)),
    cycle: source
      .cycle
      .checked_add(1)
      .context("watch cycle overflow")?,
    suite: source.suite.clone(),
    profile: source.profile.clone(),
    started_at: OffsetDateTime::from_unix_timestamp(now as i64)?.format(&Rfc3339)?,
    duration_ms: 0,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: Vec::new(),
    player_sessions: Vec::new(),
    jobs: Vec::new(),
    scenarios: Vec::new(),
    warnings: Vec::new(),
    errors: Vec::new(),
    baseline_writes: Vec::new(),
    artifacts: Vec::new(),
  })
}

fn set_identity(result: &mut RunResult, source: &RunResult, run_id: &str, now: u64) -> Result<()> {
  result.run_id = run_id.to_owned();
  result.source_run_id = Some(source.run_id.clone());
  result.command = ResultCommand::ComparisonOnly;
  result.source_command = source.source_command.or(Some(source.command));
  result.cycle = source
    .cycle
    .checked_add(1)
    .context("watch cycle overflow")?;
  result.started_at = OffsetDateTime::from_unix_timestamp(now as i64)?.format(&Rfc3339)?;
  Ok(())
}

fn normalize_capture(result: &mut RunResult) {
  for scenario in &mut result.scenarios {
    for step in &mut scenario.steps {
      if let Some(ScreenshotResult::Captured { baseline, .. }) = &mut step.screenshot
        && matches!(baseline, BaselineOutcome::NotLoaded)
      {
        *baseline = BaselineOutcome::Missing;
      }
    }
  }
}

fn comparison_settings(
  suite: &Suite,
  scenario_name: &str,
  checkpoint: &str,
  existing: Option<&ComparisonOutcome>,
) -> Result<Comparison> {
  if let Some(existing) = existing {
    return Ok(match existing {
      ComparisonOutcome::Passed { settings, .. } | ComparisonOutcome::Mismatch { settings, .. } => {
        settings.clone()
      }
    });
  }
  let authored = suite
    .scenarios
    .iter()
    .find(|scenario| scenario.name == scenario_name)
    .and_then(|scenario| {
      scenario.steps.iter().find_map(|step| match &step.action {
        StepKind::Screenshot(value) if value.name == checkpoint => Some(&value.comparison),
        _ => None,
      })
    })
    .context("selected checkpoint comparison settings are absent")?;
  Ok(Comparison {
    threshold: authored.threshold.as_str().to_owned(),
    anti_alias: authored.anti_alias,
    max_changed_percent: authored.max_changed_percent.as_str().to_owned(),
  })
}

fn reduce_status(result: &mut RunResult, source_status: RunStatus) {
  if source_status == RunStatus::InfrastructureError
    || result
      .scenarios
      .iter()
      .any(|scenario| scenario.status == ScenarioStatus::InfrastructureError)
  {
    result.status = RunStatus::InfrastructureError;
    result.exit_code = 2;
  } else if source_status == RunStatus::Interrupted
    || result
      .scenarios
      .iter()
      .any(|scenario| scenario.status == ScenarioStatus::Interrupted)
  {
    result.status = RunStatus::Interrupted;
    result.exit_code = 130;
  } else if result
    .scenarios
    .iter()
    .any(|scenario| scenario.status == ScenarioStatus::Failed)
  {
    result.status = RunStatus::Failed;
    result.exit_code = 1;
  } else {
    result.status = RunStatus::Passed;
    result.exit_code = 0;
  }
}

fn copy_baseline(source: &Path, destination: &Path) -> Result<()> {
  if source == destination {
    return Ok(());
  }
  if let Some(parent) = destination.parent() {
    fs::create_dir_all(parent)?;
  }
  if !destination.is_file() {
    fs::copy(source, destination)?;
  }
  Ok(())
}
