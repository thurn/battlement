//! Player session, lifecycle, artifact, and log wire models.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::wire::{
  common::{AssertionResult, DeadlineKind, ErrorCode, ErrorSource, StepName, StepStatus},
  completion_validation,
  job::{Capability, Display, Job, Platform},
  lifecycle_validation, log_validation,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus {
  Passed,
  Failed,
  Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoundaryStage {
  Destroy,
  Reset,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NextAction {
  Continue,
  Stop,
  Relaunch,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalReason {
  Completed,
  Bail,
  InfrastructureError,
  Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DittoLogSource {
  Battlement,
  Rust,
  Unity,
  DittoPlayer,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DittoLogSeverity {
  Trace,
  Debug,
  Information,
  Warning,
  Error,
}

/// The player's initial job acknowledgement.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Started {
  pub job_id: String,
  pub run_id: String,
  pub player_session_id: String,
  pub first_log_sequence: Option<u64>,
  pub startup_failure: Option<PlayerInfrastructureFailure>,
  pub startup_log_failure: Option<PlayerInfrastructureFailure>,
  pub identity: StartupIdentity,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StartupIdentity {
  Report(StartupReportIdentity),
  Accepted(AcceptedPlayerSessionIdentity),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupReportIdentity {
  pub startup_report: StartupReport,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPlayerSessionIdentity {
  pub accepted_player_session_id: String,
}

/// Facts reported by a newly launched player.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StartupReport {
  pub platform: Platform,
  pub capture_adapter: String,
  pub build_fingerprint: String,
  pub source_fingerprint: String,
  pub unity_version: String,
  pub diagnostics: bool,
  pub display: Display,
  pub capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogBatchAck {
  pub player_session_id: String,
  pub next_sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerInfrastructureFailure {
  pub code: ErrorCode,
  pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAck {
  pub artifact_id: String,
  pub sha256: String,
}

/// The player's durable completion payload for one reached scenario.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioComplete {
  pub scenario_id: String,
  pub execution_status: ExecutionStatus,
  pub steps: Vec<PlayerStepResult>,
  pub artifacts: Vec<ReachedArtifact>,
  pub failure_frame: Option<PlayerFailureFrame>,
  pub video_inputs: Vec<NativeVideoInput>,
  pub last_log_sequence: u64,
  pub execution_duration_ms: u64,
  pub startup_duration_ms: u64,
  pub boundary: ScenarioBoundaryOutcome,
  pub primary_error_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerStepResult {
  pub index: u32,
  pub name: Option<String>,
  pub kind: StepName,
  pub status: StepStatus,
  pub duration_ms: u64,
  pub expired_deadline: Option<DeadlineKind>,
  pub error_refs: Vec<String>,
  pub assertion: Option<AssertionResult>,
  pub screenshot_artifact_id: Option<String>,
  pub video_input_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReachedArtifact {
  pub artifact_id: String,
  pub step_index: Option<u32>,
  pub kind: ArtifactKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ArtifactKind {
  Screenshot { checkpoint: String },
  FailureFrame,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum PlayerFailureFrame {
  Captured {
    artifact_id: String,
  },
  Unavailable {
    reason: String,
    error_ref: Option<String>,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NativeVideoInput {
  pub input_id: String,
  pub start_step_index: u32,
  pub path: String,
  pub sha256: String,
  pub width: u32,
  pub height: u32,
  pub frame_count: u64,
  pub truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScenarioBoundaryOutcome {
  Passed {
    duration_ms: u64,
  },
  Failed {
    duration_ms: u64,
    stage: BoundaryStage,
    error_ref: String,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioDecision {
  pub action: NextAction,
  pub completed_failures: u32,
  pub error_id: Option<String>,
  pub error_code: Option<ErrorCode>,
  pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobComplete {
  pub job_id: String,
  pub last_log_sequence: u64,
  pub executed_scenario_ids: Vec<String>,
  pub unstarted_scenarios: Vec<UnstartedScenario>,
  pub reason: TerminalReason,
  pub execution_duration_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobCompleteAck {
  pub job_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobFailed {
  pub job_id: String,
  pub failure: PlayerInfrastructureFailure,
  pub last_log_sequence: Option<u64>,
  pub executed_scenario_ids: Vec<String>,
  pub unstarted_scenarios: Vec<UnstartedScenario>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobFailedAck {
  pub job_id: String,
  pub error_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UnstartedScenario {
  pub scenario_id: String,
  pub reason: String,
}

/// A lossless typed context entry in the unified log stream.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DittoContextRecord {
  pub schema: u32,
  pub job_id: String,
  pub player_session_id: String,
  pub sequence: u64,
  pub timestamp_unix_us: i64,
  pub source: DittoLogSource,
  pub severity: DittoLogSeverity,
  pub event_name: String,
  pub message: String,
  pub body: DittoContext,
}

/// An ordinary application entry in the unified log stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DittoLogRecord {
  pub schema: u32,
  pub job_id: String,
  pub player_session_id: String,
  pub sequence: u64,
  pub timestamp_unix_us: i64,
  pub source: DittoLogSource,
  pub severity: DittoLogSeverity,
  pub event_name: String,
  pub message: String,
  pub fields: BTreeMap<String, String>,
  pub exception: Option<String>,
  pub stack_trace: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DittoEventRecord {
  Context(DittoContextRecord),
  Log(DittoLogRecord),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "context", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DittoContext {
  JobStarted {
    run_id: String,
  },
  JobEnded {
    reason: TerminalReason,
  },
  EngineStarted {
    engine_session_id: String,
    scenario_id: String,
  },
  EngineEnded {
    engine_session_id: String,
    status: ExecutionStatus,
  },
  ScenarioStarted {
    scenario_id: String,
  },
  ScenarioEnded {
    scenario_id: String,
    execution_status: ExecutionStatus,
    failure_frame: Option<PlayerFailureFrame>,
    video_inputs: Vec<NativeVideoInput>,
    execution_duration_ms: u64,
    startup_duration_ms: u64,
    boundary: ScenarioBoundaryOutcome,
    primary_error_ref: Option<String>,
  },
  StepStarted {
    scenario_id: String,
    step_index: u32,
  },
  StepEnded {
    scenario_id: String,
    result: PlayerStepResult,
  },
  ArtifactAccepted {
    scenario_id: String,
    step_index: Option<u32>,
    artifact_id: String,
    artifact_kind: ArtifactKind,
  },
  ErrorObserved {
    scenario_id: String,
    step_index: Option<u32>,
    error_ref: String,
    code: ErrorCode,
    source: ErrorSource,
    record_sequence: Option<u64>,
    battlement_error_id: Option<String>,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpError {
  pub error_id: String,
  pub code: ErrorCode,
  pub message: String,
  pub expected_sequence: Option<u64>,
  pub related_run_id: Option<String>,
}

impl Started {
  /// Validates startup ownership and identity against the assigned job.
  pub fn validate(
    &self,
    job: &Job,
    expected_player_session_id: &str,
    accepted_player_session_id: Option<&str>,
  ) -> anyhow::Result<()> {
    lifecycle_validation::started(
      self,
      job,
      expected_player_session_id,
      accepted_player_session_id,
    )
  }
}

impl ScenarioComplete {
  /// Validates completion references against its resolved scenario.
  pub fn validate(&self, job: &Job, observed_error_refs: &[String]) -> anyhow::Result<()> {
    completion_validation::scenario_complete(self, job, observed_error_refs)
  }
}

impl LogBatchAck {
  /// Validates ownership and the next expected log sequence.
  pub fn validate(
    &self,
    player_session_id: &str,
    expected_next_sequence: u64,
  ) -> anyhow::Result<()> {
    lifecycle_validation::log_ack(self, player_session_id, expected_next_sequence)
  }
}

impl ArtifactAck {
  /// Validates ownership and content identity for an artifact upload.
  pub fn validate(&self, artifact_id: &str, sha256: &str) -> anyhow::Result<()> {
    lifecycle_validation::artifact_ack(self, artifact_id, sha256)
  }
}

impl ScenarioDecision {
  /// Validates the conditional error response fields.
  pub fn validate(&self) -> anyhow::Result<()> {
    lifecycle_validation::scenario_decision(self)
  }
}

impl JobComplete {
  /// Validates terminal scenario accounting against the assigned job.
  pub fn validate(&self, job: &Job) -> anyhow::Result<()> {
    lifecycle_validation::job_complete(self, job)
  }
}

impl JobFailed {
  /// Validates failed terminal scenario accounting against the assigned job.
  pub fn validate(&self, job: &Job) -> anyhow::Result<()> {
    lifecycle_validation::job_failed(self, job)
  }
}

impl JobCompleteAck {
  /// Validates acknowledgement ownership.
  pub fn validate(&self, job_id: &str) -> anyhow::Result<()> {
    lifecycle_validation::complete_ack(self, job_id)
  }
}

impl JobFailedAck {
  /// Validates acknowledgement ownership and its allocated error occurrence.
  pub fn validate(&self, job_id: &str) -> anyhow::Result<()> {
    lifecycle_validation::failed_ack(self, job_id)
  }
}

impl HttpError {
  /// Validates stable error IDs and conditional transport fields.
  pub fn validate(&self) -> anyhow::Result<()> {
    lifecycle_validation::http_error(self)
  }
}

/// Parses and validates one exact contiguous NDJSON request body.
pub fn decode_ndjson(
  bytes: &[u8],
  job: &Job,
  player_session_id: &str,
  first_sequence: u64,
) -> anyhow::Result<Vec<DittoEventRecord>> {
  log_validation::decode_ndjson(bytes, job, player_session_id, first_sequence)
}

impl<'de> Deserialize<'de> for ArtifactKind {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Ok(match RawArtifactKind::deserialize(deserializer)? {
      RawArtifactKind::Screenshot { checkpoint } => Self::Screenshot { checkpoint },
      RawArtifactKind::FailureFrame {} => Self::FailureFrame,
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum RawArtifactKind {
  Screenshot { checkpoint: String },
  FailureFrame {},
}
