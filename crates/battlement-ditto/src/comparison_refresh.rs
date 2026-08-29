use std::{
  collections::BTreeSet,
  fs,
  io::Write,
  path::{Path, PathBuf},
  sync::Arc,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use battlement_tooling::discovery::HostDiscovery;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
  baseline_manifest::ManifestSnapshot,
  baseline_store::{self, ReachedComparison, ReachedComparisonRequest},
  config::model::{StepKind, Suite, Target},
  image_comparison::{OdiffPool, OdiffServer},
  maintenance_commands, storage_commands,
  wire::{
    common::{ErrorCode, ErrorSource, StepStatus},
    job::Comparison,
    result::{
      BaselineOutcome, ComparisonOutcome, ErrorOccurrence, ImageFile, PhaseName, PhaseResult,
      PhaseStatus, ResultCommand, RunResult, RunStatus, ScenarioStatus, ScreenshotResult,
    },
    run_storage::{ActiveRun, RunStore},
  },
};

pub(crate) struct RefreshedComparison {
  pub result: RunResult,
  pub result_path: PathBuf,
  pub directory: PathBuf,
}

pub(crate) fn refresh(
  suite: &Suite,
  source: &RunResult,
  cycle: u32,
  pool: Arc<OdiffPool>,
  stderr: &mut dyn Write,
) -> Result<RefreshedComparison> {
  let started = Instant::now();
  let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
  let roots = maintenance_commands::cache_roots(suite)?;
  let mut store = RunStore::open(&roots.runs)?;
  let mut result = derived_result(source, cycle, now)?;
  let mut active = store.begin(result.clone(), stderr, now)?;
  store.index_identity(&active, &suite.repository, &suite.name, now)?;
  store.materialize_derived(&active, &source.run_id, &source.artifacts, now)?;
  let snapshot = ManifestSnapshot::read(&storage_commands::lock_path(suite))?;
  result.lock_sha256 = snapshot.sha256.clone();
  let discovery = HostDiscovery::inspect(
    &battlement_tooling::host::SystemHost,
    &maintenance_commands::discovery_request(suite, Target::Macos)?,
  )?;
  let odiff = discovery.odiff.path.context("ODiff is unavailable")?;
  let baseline = storage_commands::read_store(suite)?;
  remove_comparison_errors(&mut result);
  let profile = result
    .profile
    .clone()
    .context("comparison profile is absent")?;
  let mut errors = std::mem::take(&mut result.errors);
  let timeout = Duration::from_millis(suite.timeouts.baseline_download.as_millis());
  pool.with_server(
    &odiff,
    &active.path().join("odiff.log"),
    timeout,
    |server| {
      compare_screenshots(
        &mut CompareContext {
          suite,
          profile: &profile,
          snapshot: &snapshot,
          baseline: baseline.as_ref(),
          cache: &roots.baselines,
          server,
          active: &active,
          errors: &mut errors,
        },
        &mut result.scenarios,
      )
    },
  )?;
  result.errors = errors;
  reduce(&mut result);
  result.duration_ms = started.elapsed().as_millis() as u64;
  result.phases = vec![PhaseResult {
    name: PhaseName::Comparison,
    status: if result.status == RunStatus::InfrastructureError {
      PhaseStatus::Failed
    } else {
      PhaseStatus::Passed
    },
    duration_ms: result.duration_ms,
    expired_deadline: None,
    log_path: None,
    error_ids: Vec::new(),
  }];
  let result_path = store.finalize(&mut active, result, now)?;
  let result: RunResult = serde_json::from_slice(&fs::read(&result_path)?)?;
  Ok(RefreshedComparison {
    directory: active.path().to_path_buf(),
    result,
    result_path,
  })
}

struct CompareContext<'a> {
  suite: &'a Suite,
  profile: &'a str,
  snapshot: &'a ManifestSnapshot,
  baseline: &'a dyn baseline_store::BaselineStore,
  cache: &'a Path,
  server: &'a mut OdiffServer,
  active: &'a ActiveRun,
  errors: &'a mut Vec<ErrorOccurrence>,
}

fn compare_screenshots(
  context: &mut CompareContext<'_>,
  scenarios: &mut [crate::wire::result::ScenarioResult],
) -> Result<()> {
  let timeout = Duration::from_millis(context.suite.timeouts.baseline_download.as_millis());
  for scenario in scenarios {
    for step in &mut scenario.steps {
      let Some(ScreenshotResult::Captured {
        checkpoint,
        actual,
        baseline: baseline_outcome,
        comparison,
        matched_before_update,
        updated,
      }) = &mut step.screenshot
      else {
        continue;
      };
      *matched_before_update = None;
      *updated = None;
      step.error_ids.clear();
      let diff = context
        .active
        .path()
        .join("diffs")
        .join(format!("{}.png", actual.sha256));
      fs::create_dir_all(diff.parent().unwrap())?;
      let compared = baseline_store::compare_reached(
        context.baseline,
        context.snapshot.manifest.as_ref(),
        context.cache,
        context.server,
        ReachedComparisonRequest {
          profile: context.profile,
          scenario: &scenario.name,
          checkpoint,
          actual: &context.active.path().join(&actual.path),
          diff: &diff,
          settings: settings(
            context.suite,
            &scenario.name,
            checkpoint,
            comparison.as_ref(),
          )?,
          timeout,
        },
      )?;
      match compared {
        ReachedComparison::Missing => {
          *baseline_outcome = BaselineOutcome::Missing;
          *comparison = None;
          fail_step(
            context.errors,
            step,
            &scenario.id,
            ErrorCode::ImageMissingBaseline,
            "baseline is missing",
          )?;
        }
        ReachedComparison::Compared {
          entry,
          baseline,
          comparison: compared,
        } => {
          let relative = format!("baselines/{}.png", entry.sha256);
          copy(&baseline, &context.active.path().join(&relative))?;
          *baseline_outcome = BaselineOutcome::Loaded {
            image: ImageFile {
              path: relative,
              sha256: entry.sha256,
              width: entry.width,
              height: entry.height,
            },
          };
          let mut outcome = compared.outcome;
          if let ComparisonOutcome::Mismatch { diff, .. } = &mut outcome {
            diff.path = format!("diffs/{}.png", actual.sha256);
          }
          *comparison = Some(outcome);
          if matches!(comparison, Some(ComparisonOutcome::Mismatch { .. })) {
            fail_step(
              context.errors,
              step,
              &scenario.id,
              ErrorCode::ImageMismatch,
              "image comparison mismatched",
            )?;
          } else {
            step.status = StepStatus::Passed;
          }
        }
      }
    }
  }
  Ok(())
}

fn settings(
  suite: &Suite,
  scenario: &str,
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
  let value = suite
    .scenarios
    .iter()
    .find(|value| value.name == scenario)
    .and_then(|value| {
      value.steps.iter().find_map(|step| match &step.action {
        StepKind::Screenshot(value) if value.name == checkpoint => Some(&value.comparison),
        _ => None,
      })
    })
    .context("screenshot comparison settings are absent")?;
  Ok(Comparison {
    threshold: value.threshold.as_str().to_owned(),
    anti_alias: value.anti_alias,
    max_changed_percent: value.max_changed_percent.as_str().to_owned(),
  })
}

fn fail_step(
  errors: &mut Vec<ErrorOccurrence>,
  step: &mut crate::wire::result::StepResult,
  scenario_id: &str,
  code: ErrorCode,
  message: &str,
) -> Result<()> {
  let number = errors.len() + 1;
  anyhow::ensure!(number <= 9999, "run error occurrence limit exceeded");
  let id = format!("E{number:04}");
  errors.push(ErrorOccurrence {
    id: id.clone(),
    code,
    source: ErrorSource::ODiff,
    message: message.to_owned(),
    job_id: None,
    player_session_id: None,
    scenario_id: Some(scenario_id.to_owned()),
    step_index: Some(step.index),
    log_sequence: None,
  });
  step.error_ids.push(id);
  step.status = StepStatus::Failed;
  Ok(())
}

fn remove_comparison_errors(result: &mut RunResult) {
  let removed: BTreeSet<_> = result
    .scenarios
    .iter()
    .flat_map(|scenario| scenario.steps.iter())
    .filter(|step| step.screenshot.is_some())
    .flat_map(|step| step.error_ids.iter().cloned())
    .collect();
  result.errors.retain(|error| !removed.contains(&error.id));
}

fn derived_result(source: &RunResult, cycle: u32, now: u64) -> Result<RunResult> {
  let mut result = source.clone();
  result.run_id = Uuid::new_v4().to_string();
  result.source_run_id = Some(source.run_id.clone());
  result.command = ResultCommand::ComparisonOnly;
  result.source_command = source.source_command.or(Some(source.command));
  result.cycle = cycle;
  result.started_at = OffsetDateTime::from_unix_timestamp(now as i64)?.format(&Rfc3339)?;
  result.build = None;
  result.player_sessions.clear();
  result.jobs.clear();
  result.baseline_writes.clear();
  result.artifacts.clear();
  Ok(result)
}

fn reduce(result: &mut RunResult) {
  for scenario in &mut result.scenarios {
    scenario.status = if scenario
      .steps
      .iter()
      .any(|step| step.status == StepStatus::InfrastructureError)
    {
      ScenarioStatus::InfrastructureError
    } else if scenario
      .steps
      .iter()
      .any(|step| step.status == StepStatus::Failed)
    {
      ScenarioStatus::Failed
    } else if scenario.status != ScenarioStatus::Skipped {
      ScenarioStatus::Passed
    } else {
      ScenarioStatus::Skipped
    };
  }
  if result
    .scenarios
    .iter()
    .any(|scenario| scenario.status == ScenarioStatus::InfrastructureError)
  {
    result.status = RunStatus::InfrastructureError;
    result.exit_code = 2;
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

fn copy(source: &Path, destination: &Path) -> Result<()> {
  fs::create_dir_all(
    destination
      .parent()
      .context("baseline path has no parent")?,
  )?;
  if !destination.is_file() {
    fs::copy(source, destination)?;
  }
  Ok(())
}
