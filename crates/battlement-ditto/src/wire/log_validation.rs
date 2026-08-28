use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};

use crate::wire::{
  completion_validation,
  job::{Job, ResolvedScenario, StepKind},
  lifecycle::{
    ArtifactKind, DittoContext, DittoContextRecord, DittoEventRecord, DittoLogRecord,
    ScenarioBoundaryOutcome,
  },
  lifecycle_validation, validation,
};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;

pub(super) fn decode_ndjson(
  bytes: &[u8],
  job: &Job,
  player_session_id: &str,
  first_sequence: u64,
) -> Result<Vec<DittoEventRecord>> {
  ensure!(!bytes.is_empty(), "NDJSON body must not be empty");
  ensure!(
    bytes.len() <= MAX_REQUEST_BYTES,
    "NDJSON body exceeds 1 MiB"
  );
  let source = std::str::from_utf8(bytes).context("NDJSON body must be UTF-8")?;
  let Some(source) = source.strip_suffix('\n') else {
    anyhow::bail!("NDJSON body must end with LF");
  };
  ensure!(!source.is_empty(), "NDJSON body must contain a record");
  let mut records = Vec::new();
  for (offset, line) in source.split('\n').enumerate() {
    ensure!(!line.is_empty(), "NDJSON body must not contain blank lines");
    ensure!(
      line.len() < MAX_REQUEST_BYTES,
      "one NDJSON record exceeds 1 MiB"
    );
    let record: DittoEventRecord = serde_json::from_str(line)
      .with_context(|| format!("invalid NDJSON record at line {}", offset + 1))?;
    let sequence = first_sequence
      .checked_add(offset as u64)
      .ok_or_else(|| anyhow::anyhow!("log sequence overflow"))?;
    event_record(&record, job, player_session_id, sequence)?;
    records.push(record);
  }
  Ok(records)
}

fn event_record(
  record: &DittoEventRecord,
  job: &Job,
  player_session_id: &str,
  expected_sequence: u64,
) -> Result<()> {
  match record {
    DittoEventRecord::Context(record) => {
      common_record(
        record.schema,
        &record.job_id,
        &record.player_session_id,
        record.sequence,
        &record.event_name,
        &record.message,
        job,
        player_session_id,
        expected_sequence,
      )?;
      context(record, job)
    }
    DittoEventRecord::Log(record) => {
      common_record(
        record.schema,
        &record.job_id,
        &record.player_session_id,
        record.sequence,
        &record.event_name,
        &record.message,
        job,
        player_session_id,
        expected_sequence,
      )?;
      log_record(record)
    }
  }
}

#[allow(clippy::too_many_arguments)]
fn common_record(
  schema: u32,
  job_id: &str,
  record_player_session_id: &str,
  sequence: u64,
  event_name: &str,
  message: &str,
  job: &Job,
  player_session_id: &str,
  expected_sequence: u64,
) -> Result<()> {
  ensure!(schema == 1, "log schema must equal 1");
  ensure!(job_id == job.job_id, "log record belongs to another job");
  validation::identifier("player_session_id", record_player_session_id)?;
  ensure!(
    record_player_session_id == player_session_id,
    "log record belongs to another player session"
  );
  ensure!(
    sequence == expected_sequence,
    "log records must have contiguous sequences"
  );
  validation::name("event_name", event_name)?;
  ensure!(
    message.len() <= 4096,
    "log message may contain at most 4096 UTF-8 bytes"
  );
  Ok(())
}

fn log_record(record: &DittoLogRecord) -> Result<()> {
  ensure!(
    record.fields.len() <= 128,
    "log fields may contain at most 128 entries"
  );
  for (key, value) in &record.fields {
    validation::name("log field name", key)?;
    ensure!(
      value.len() <= 4096,
      "log field value may contain at most 4096 UTF-8 bytes"
    );
  }
  Ok(())
}

fn context(record: &DittoContextRecord, job: &Job) -> Result<()> {
  match &record.body {
    DittoContext::JobStarted { run_id } => {
      ensure!(
        run_id == &job.run_id,
        "job-started context has the wrong run_id"
      );
    }
    DittoContext::JobEnded { .. } => {}
    DittoContext::EngineStarted {
      engine_session_id,
      scenario_id,
    } => {
      validation::identifier("engine_session_id", engine_session_id)?;
      lifecycle_validation::scenario(job, scenario_id)?;
    }
    DittoContext::EngineEnded {
      engine_session_id, ..
    } => validation::identifier("engine_session_id", engine_session_id)?,
    DittoContext::ScenarioStarted { scenario_id } => {
      lifecycle_validation::scenario(job, scenario_id)?;
    }
    DittoContext::ScenarioEnded {
      scenario_id,
      failure_frame,
      video_inputs,
      primary_error_ref,
      boundary,
      ..
    } => {
      let scenario = lifecycle_validation::scenario(job, scenario_id)?;
      if let Some(frame) = failure_frame {
        lifecycle_validation::failure_frame(frame)?;
      }
      ensure!(
        video_inputs.len() <= 64,
        "scenario context may contain at most 64 video inputs"
      );
      let mut input_ids = BTreeSet::new();
      for input in video_inputs {
        lifecycle_validation::native_video(input, scenario)?;
        ensure!(
          input_ids.insert(&input.input_id),
          "scenario context video IDs must be unique"
        );
      }
      if let Some(error_ref) = primary_error_ref {
        lifecycle_validation::player_error_ref(error_ref)?;
      }
      if let ScenarioBoundaryOutcome::Failed { error_ref, .. } = boundary {
        lifecycle_validation::player_error_ref(error_ref)?;
      }
    }
    DittoContext::StepStarted {
      scenario_id,
      step_index,
    } => {
      let scenario = lifecycle_validation::scenario(job, scenario_id)?;
      ensure!(
        (*step_index as usize) < scenario.steps.len(),
        "step-started index is outside the scenario"
      );
    }
    DittoContext::StepEnded {
      scenario_id,
      result,
    } => {
      let scenario = lifecycle_validation::scenario(job, scenario_id)?;
      let Some(step) = scenario.steps.get(result.index as usize) else {
        anyhow::bail!("step-ended index is outside the scenario");
      };
      completion_validation::step_result(step, result)?;
    }
    DittoContext::ArtifactAccepted {
      scenario_id,
      step_index,
      artifact_id,
      artifact_kind,
    } => {
      let scenario = lifecycle_validation::scenario(job, scenario_id)?;
      validation::identifier("artifact_id", artifact_id)?;
      lifecycle_validation::artifact_kind(artifact_kind)?;
      artifact_step(scenario, *step_index, artifact_kind)?;
    }
    DittoContext::ErrorObserved {
      scenario_id,
      step_index,
      error_ref,
      record_sequence,
      battlement_error_id,
      ..
    } => {
      let scenario = lifecycle_validation::scenario(job, scenario_id)?;
      if let Some(index) = step_index {
        ensure!(
          (*index as usize) < scenario.steps.len(),
          "error step index is outside scenario"
        );
      }
      lifecycle_validation::player_error_ref(error_ref)?;
      if let Some(sequence) = record_sequence {
        ensure!(
          *sequence < record.sequence,
          "observed error must follow its source record"
        );
      }
      if let Some(error_id) = battlement_error_id {
        validation::name("battlement_error_id", error_id)?;
      }
    }
  }
  Ok(())
}

fn artifact_step(
  scenario: &ResolvedScenario,
  step_index: Option<u32>,
  kind: &ArtifactKind,
) -> Result<()> {
  match kind {
    ArtifactKind::Screenshot { checkpoint } => {
      let Some(index) = step_index else {
        anyhow::bail!("screenshot artifact context requires step_index");
      };
      let Some(step) = scenario.steps.get(index as usize) else {
        anyhow::bail!("artifact step_index is outside scenario");
      };
      let StepKind::Screenshot(expected) = &step.action else {
        anyhow::bail!("screenshot artifact context must reference a screenshot step");
      };
      ensure!(
        checkpoint == &expected.name,
        "artifact checkpoint does not match the job"
      );
    }
    ArtifactKind::FailureFrame => {
      if let Some(index) = step_index {
        ensure!(
          (index as usize) < scenario.steps.len(),
          "failure frame step is outside scenario"
        );
      }
    }
  }
  Ok(())
}
