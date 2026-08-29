//! Host materialization of reached player scenarios and screenshots.

use std::{
  collections::BTreeMap,
  fs,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  sync::{Arc, Mutex},
  time::Duration,
};

use crate::{
  baseline_manifest::BaselineManifest,
  baseline_store::{self, BaselineStore, ReachedBaseline},
  baseline_update::BaselineProposal,
  execution_artifacts,
  image_comparison::{ImageComparisonRequest, OdiffPool},
  scenario_orchestration::{DecisionFailure, MaterializedScenario, ScenarioMaterializer},
  wire::{
    common::{ErrorCode, ErrorSource, StepStatus},
    job::{Job, StepKind},
    lifecycle::{DittoContext, DittoEventRecord, PlayerStepResult, ScenarioComplete},
    result::{
      BaselineOutcome, ComparisonOutcome, ErrorOccurrence, LogSpan, Recovery, ResultCommand,
      ScenarioResult, ScenarioTimings, ScreenshotResult, StepResult,
    },
  },
};
use anyhow::{Context, Result};

pub(crate) struct ExecutionMaterializer {
  run_directory: PathBuf,
  profile: String,
  command: ResultCommand,
  manifest: Option<BaselineManifest>,
  store: Option<Box<dyn BaselineStore>>,
  baseline_cache: PathBuf,
  odiff_binary: Option<PathBuf>,
  odiff: Arc<OdiffPool>,
  comparison_timeout: Duration,
  source_fingerprint: String,
  state: Mutex<State>,
}

pub(crate) struct Options {
  pub run_directory: PathBuf,
  pub profile: String,
  pub command: ResultCommand,
  pub manifest: Option<BaselineManifest>,
  pub store: Option<Box<dyn BaselineStore>>,
  pub baseline_cache: PathBuf,
  pub odiff_binary: Option<PathBuf>,
  pub odiff: Option<Arc<OdiffPool>>,
  pub comparison_timeout: Duration,
  pub source_fingerprint: String,
}

#[derive(Default)]
struct State {
  errors: Vec<ErrorOccurrence>,
  proposals: Vec<BaselineProposal>,
}

struct ObservedError {
  code: ErrorCode,
  source: ErrorSource,
  message: String,
  job_id: String,
  player_session_id: String,
  scenario_id: String,
  step_index: Option<u32>,
  sequence: u64,
}

impl ExecutionMaterializer {
  pub(crate) fn new(options: Options) -> Self {
    Self {
      run_directory: options.run_directory,
      profile: options.profile,
      command: options.command,
      manifest: options.manifest,
      store: options.store,
      baseline_cache: options.baseline_cache,
      odiff_binary: options.odiff_binary,
      odiff: options.odiff.unwrap_or_default(),
      comparison_timeout: options.comparison_timeout,
      source_fingerprint: options.source_fingerprint,
      state: Mutex::new(State::default()),
    }
  }

  pub(crate) fn errors(&self) -> Vec<ErrorOccurrence> {
    self.state.lock().unwrap().errors.clone()
  }

  pub(crate) fn proposals(&self) -> Vec<BaselineProposal> {
    self.state.lock().unwrap().proposals.clone()
  }

  fn materialize_step(
    &self,
    job: &Job,
    scenario_name: &str,
    scenario_id: &str,
    player: &PlayerStepResult,
    observations: &BTreeMap<String, ObservedError>,
    state: &mut State,
  ) -> Result<(StepResult, Option<DecisionFailure>)> {
    let mut failure = None;
    let mut error_ids = Vec::new();
    for error_ref in &player.error_refs {
      let observed = observations
        .get(error_ref)
        .with_context(|| format!("player error reference {error_ref} has no log context"))?;
      let error_id = allocate_error(state, observed);
      failure.get_or_insert_with(|| DecisionFailure {
        error_id: error_id.clone(),
        code: observed.code,
        message: observed.message.clone(),
      });
      error_ids.push(error_id);
    }
    let mut status = player.status;
    let mut screenshot = None;
    if let Some(artifact_id) = &player.screenshot_artifact_id {
      let checkpoint = match &job
        .scenarios
        .iter()
        .find(|scenario| scenario.id == scenario_id)
        .context("completed scenario is absent from its job")?
        .steps[player.index as usize]
        .action
      {
        StepKind::Screenshot(value) => value.name.clone(),
        _ => anyhow::bail!("non-screenshot step referenced a screenshot artifact"),
      };
      match self.screenshot(
        state,
        scenario_name,
        player.index,
        &checkpoint,
        artifact_id,
        job,
      ) {
        Ok((value, functional_failure)) => {
          if functional_failure {
            status = StepStatus::Failed;
            let observed = host_error(
              ErrorCode::ImageMismatch,
              "screenshot did not match an accepted baseline",
              &job.job_id,
              scenario_id,
              player.index,
            );
            let error_id = allocate_error(state, &observed);
            error_ids.push(error_id.clone());
            failure.get_or_insert(DecisionFailure {
              error_id,
              code: observed.code,
              message: observed.message,
            });
          }
          screenshot = Some(value);
        }
        Err(error) => {
          status = StepStatus::InfrastructureError;
          let observed = host_error(
            ErrorCode::ImageComparisonFailed,
            &format!("{error:#}"),
            &job.job_id,
            scenario_id,
            player.index,
          );
          let error_id = allocate_error(state, &observed);
          error_ids.push(error_id.clone());
          failure.get_or_insert(DecisionFailure {
            error_id: error_id.clone(),
            code: observed.code,
            message: observed.message.clone(),
          });
          screenshot = Some(ScreenshotResult::Unavailable {
            reason: observed.message,
            error_id,
          });
        }
      }
    }
    Ok((
      StepResult {
        index: player.index,
        name: player.name.clone(),
        kind: player.kind,
        status,
        status_reason: None,
        duration_ms: player.duration_ms,
        expired_deadline: player.expired_deadline,
        error_ids,
        assertion: player.assertion.clone(),
        screenshot,
        video: None,
      },
      failure,
    ))
  }

  fn screenshot(
    &self,
    state: &mut State,
    scenario_name: &str,
    _step_index: u32,
    checkpoint: &str,
    artifact_id: &str,
    job: &Job,
  ) -> Result<(ScreenshotResult, bool)> {
    let relative_actual = format!("artifacts/{artifact_id}.png");
    let actual_path = self.run_directory.join(&relative_actual);
    let actual = execution_artifacts::image_file(&actual_path, relative_actual)?;
    state.proposals.push(BaselineProposal::from_png(
      scenario_name.to_owned(),
      checkpoint.to_owned(),
      actual_path.clone(),
      self.source_fingerprint.clone(),
    )?);
    if self.command == ResultCommand::Capture {
      return Ok((
        ScreenshotResult::Captured {
          checkpoint: checkpoint.to_owned(),
          actual,
          baseline: BaselineOutcome::NotLoaded,
          comparison: None,
          matched_before_update: None,
          updated: None,
        },
        false,
      ));
    }
    let Some(store) = self.store.as_deref() else {
      return Ok((
        execution_artifacts::missing_screenshot(checkpoint, actual),
        true,
      ));
    };
    let reached = baseline_store::hydrate_reached(
      store,
      self.manifest.as_ref(),
      &self.baseline_cache,
      &self.profile,
      scenario_name,
      checkpoint,
    )?;
    let ReachedBaseline::Hydrated { entry, path } = reached else {
      return Ok((
        execution_artifacts::missing_screenshot(checkpoint, actual),
        true,
      ));
    };
    let baseline_relative = format!("baselines/{}.png", entry.sha256);
    execution_artifacts::copy_artifact(&path, &self.run_directory.join(&baseline_relative))?;
    let baseline = execution_artifacts::image_file(
      &self.run_directory.join(&baseline_relative),
      baseline_relative,
    )?;
    let diff_relative = format!("comparisons/{}/{checkpoint}.png", job.job_id);
    let diff_path = self.run_directory.join(&diff_relative);
    if let Some(parent) = diff_path.parent() {
      fs::create_dir_all(parent)?;
    }
    let binary = self
      .odiff_binary
      .as_deref()
      .context("ODiff is unavailable")?;
    let comparison = Box::new(
      self.odiff.compare(
        binary,
        &self.run_directory.join("diagnostics/odiff.log"),
        self.comparison_timeout,
        ImageComparisonRequest {
          baseline: &path,
          actual: &actual_path,
          diff: &diff_path,
          settings: job
            .scenarios
            .iter()
            .find(|scenario| scenario.name == scenario_name)
            .and_then(|scenario| scenario.steps.iter().find(|step| step.index == _step_index))
            .and_then(|step| match &step.action {
              StepKind::Screenshot(value) => Some(value.comparison.clone()),
              _ => None,
            })
            .context("screenshot comparison settings are missing")?,
          timeout: self.comparison_timeout,
        },
      )?,
    );
    let mut outcome = comparison.outcome;
    if let ComparisonOutcome::Mismatch { diff, .. } = &mut outcome {
      diff.path = diff_relative;
    }
    let failed = matches!(outcome, ComparisonOutcome::Mismatch { .. });
    Ok((
      ScreenshotResult::Captured {
        checkpoint: checkpoint.to_owned(),
        actual,
        baseline: BaselineOutcome::Loaded { image: baseline },
        comparison: Some(outcome),
        matched_before_update: None,
        updated: None,
      },
      failed,
    ))
  }
}

impl ScenarioMaterializer for ExecutionMaterializer {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> Result<MaterializedScenario> {
    let expected = job
      .scenarios
      .iter()
      .find(|scenario| scenario.id == complete.scenario_id)
      .context("completed scenario is absent from its job")?;
    let records = log_records(&self.run_directory.join("logs/events.jsonl"))?;
    let observations = observed_errors(&records);
    let mut state = self.state.lock().unwrap();
    let mut steps = Vec::new();
    let mut primary_failure = None;
    for player in &complete.steps {
      let (step, failure) = self.materialize_step(
        job,
        &expected.name,
        &expected.id,
        player,
        &observations,
        &mut state,
      )?;
      if primary_failure.is_none() {
        primary_failure = failure;
      }
      steps.push(step);
    }
    let status = execution_artifacts::scenario_status(complete.execution_status, &steps);
    if primary_failure.is_none()
      && let Some(error_ref) = &complete.primary_error_ref
      && let Some(observed) = observations.get(error_ref)
    {
      primary_failure = Some(DecisionFailure {
        error_id: allocate_error(&mut state, observed),
        code: observed.code,
        message: observed.message.clone(),
      });
    }
    let (first_sequence, player_session_id) =
      execution_artifacts::scenario_log_identity(&records, &expected.id)
        .context("scenario log span is missing")?;
    Ok(MaterializedScenario {
      result: ScenarioResult {
        id: expected.id.clone(),
        name: expected.name.clone(),
        status,
        status_reason: None,
        motion: expected.motion,
        duration_ms: complete.execution_duration_ms,
        expired_deadline: execution_artifacts::scenario_deadline(&steps),
        timings: ScenarioTimings {
          startup_ms: Some(complete.startup_duration_ms),
          reset_ms: Some(execution_artifacts::boundary_duration(&complete.boundary)),
          durability_ms: Some(0),
          ..ScenarioTimings::default()
        },
        steps,
        logs: Some(LogSpan {
          job_id: job.job_id.clone(),
          player_session_id,
          first_sequence,
          last_sequence: complete.last_log_sequence,
          complete: true,
          path: "logs/events.jsonl".to_owned(),
        }),
        failure_frame: execution_artifacts::failure_frame(
          &self.run_directory,
          complete.failure_frame.as_ref(),
        )?,
        recovery,
      },
      primary_failure,
    })
  }
}

fn log_records(path: &Path) -> Result<Vec<DittoEventRecord>> {
  BufReader::new(fs::File::open(path)?)
    .lines()
    .map(|line| Ok(serde_json::from_str(&line?)?))
    .collect()
}

fn observed_errors(records: &[DittoEventRecord]) -> BTreeMap<String, ObservedError> {
  let messages = records
    .iter()
    .filter_map(|record| match record {
      DittoEventRecord::Log(record) => Some((record.sequence, record.message.clone())),
      DittoEventRecord::Context(_) => None,
    })
    .collect::<BTreeMap<_, _>>();
  records
    .iter()
    .filter_map(|record| {
      let DittoEventRecord::Context(record) = record else {
        return None;
      };
      let DittoContext::ErrorObserved {
        scenario_id,
        step_index,
        error_ref,
        code,
        source,
        record_sequence,
        ..
      } = &record.body
      else {
        return None;
      };
      Some((
        error_ref.clone(),
        ObservedError {
          code: *code,
          source: *source,
          message: record_sequence
            .and_then(|sequence| messages.get(&sequence).cloned())
            .unwrap_or_else(|| record.message.clone()),
          job_id: record.job_id.clone(),
          player_session_id: record.player_session_id.clone(),
          scenario_id: scenario_id.clone(),
          step_index: *step_index,
          sequence: record.sequence,
        },
      ))
    })
    .collect()
}

fn allocate_error(state: &mut State, observed: &ObservedError) -> String {
  let log_sequence = (!observed.player_session_id.is_empty()).then_some(observed.sequence);
  if let Some(existing) = state.errors.iter().find(|error| {
    error.job_id.as_ref() == Some(&observed.job_id)
      && error.scenario_id.as_ref() == Some(&observed.scenario_id)
      && error.step_index == observed.step_index
      && error.log_sequence == log_sequence
  }) {
    return existing.id.clone();
  }
  let id = format!("E{:04}", state.errors.len() + 1);
  state.errors.push(ErrorOccurrence {
    id: id.clone(),
    code: observed.code,
    source: observed.source,
    message: observed.message.clone(),
    job_id: Some(observed.job_id.clone()),
    player_session_id: (!observed.player_session_id.is_empty())
      .then(|| observed.player_session_id.clone()),
    scenario_id: Some(observed.scenario_id.clone()),
    step_index: observed.step_index,
    log_sequence,
  });
  id
}

fn host_error(
  code: ErrorCode,
  message: &str,
  job_id: &str,
  scenario_id: &str,
  step_index: u32,
) -> ObservedError {
  ObservedError {
    code,
    source: ErrorSource::Ditto,
    message: message.to_owned(),
    job_id: job_id.to_owned(),
    player_session_id: String::new(),
    scenario_id: scenario_id.to_owned(),
    step_index: Some(step_index),
    sequence: 0,
  }
}
