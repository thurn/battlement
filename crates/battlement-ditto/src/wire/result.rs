//! Durable result models for Ditto runs.

use anyhow::Result;
use serde::{Deserialize, Deserializer, Serialize};

use crate::wire::{
  common::{AssertionResult, DeadlineKind, ErrorCode, ErrorSource, StepName, StepStatus},
  job::{Comparison, Motion, PerformancePass},
  lifecycle::{PacingConfiguration, StartupReport, StepPerformance},
  result_format, result_validation,
};

/// The complete durable outcome of one Ditto cycle.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
  pub run_id: String,
  pub source_run_id: Option<String>,
  pub lock_sha256: Option<String>,
  pub command: ResultCommand,
  pub source_command: Option<ResultCommand>,
  pub cycle: u32,
  pub suite: Option<String>,
  pub profile: Option<String>,
  pub started_at: String,
  pub duration_ms: u64,
  pub status: RunStatus,
  pub exit_code: u8,
  pub build: Option<BuildResult>,
  pub phases: Vec<PhaseResult>,
  pub player_sessions: Vec<PlayerSessionResult>,
  pub jobs: Vec<JobResult>,
  pub scenarios: Vec<ScenarioResult>,
  pub warnings: Vec<String>,
  pub errors: Vec<ErrorOccurrence>,
  pub baseline_writes: Vec<BaselineWriteResult>,
  pub artifacts: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub performance: Option<PerformanceResult>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceResult {
  pub headline: String,
  pub timing_proxy: String,
  pub timing_limitation: String,
  pub target_fps: u32,
  pub missed_interaction_deadlines: u64,
  pub goal: u64,
  pub measured_score_attempts: u32,
  pub score_attempt_values: Vec<u64>,
  pub pacing_configurations: Vec<PacingConfiguration>,
  pub warmup_attempts: Vec<PerformanceAttemptSummary>,
  pub score_attempts: Vec<PerformanceAttemptSummary>,
  pub measured_steps: Vec<MeasuredStepPerformance>,
  pub observer_passes: Vec<ObserverPassSummary>,
  pub detail_hotspots: Vec<PerformanceHotspot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceAttemptSummary {
  pub pass: PerformancePass,
  pub iteration: u32,
  pub warmup: bool,
  pub missed_interaction_deadlines: u64,
  pub response_missed_deadlines: u64,
  pub pacing_missed_deadlines: u64,
  pub no_visual_response_attempts: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverPassSummary {
  pub pass: PerformancePass,
  pub attempts: u32,
  pub observed_frames: u64,
  pub observed_pixels: u64,
  pub stages: Vec<ObserverStageSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObserverStageSummary {
  pub stage: String,
  pub total_ns: u64,
  pub p95_ns: u64,
  pub maximum_ns: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceHotspot {
  pub component: String,
  pub calls: u64,
  pub total_self_duration_us: u64,
  pub maximum_self_duration_us: u64,
  pub total_inclusive_duration_us: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredStepPerformance {
  pub name: String,
  pub attempts: u32,
  pub no_visual_response_attempts: u32,
  pub missed_interaction_deadlines: u64,
  pub response_missed_deadlines: u64,
  pub pacing_missed_deadlines: u64,
  pub median_response_latency_ns: u64,
  pub p95_response_latency_ns: u64,
  pub maximum_response_latency_ns: u64,
  pub median_semantic_completion_latency_ns: Option<u64>,
  pub median_settled_completion_latency_ns: Option<u64>,
  pub worst_presentation_interval_ns: u64,
  pub presentation_interval_distribution: Distribution,
  pub over_budget_presentation_intervals: u64,
  pub longest_over_budget_sequence: u32,
  pub managed_allocated_bytes: Option<i64>,
  pub attempt_values: Vec<MeasuredStepAttempt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredStepAttempt {
  pub iteration: u32,
  pub response_latency_ns: u64,
  pub semantic_completion_latency_ns: Option<u64>,
  pub settled_completion_latency_ns: Option<u64>,
  pub response_missed_deadlines: u64,
  pub pacing_missed_deadlines: u64,
  pub missed_interaction_deadlines: u64,
  pub no_visual_response: bool,
  pub presented_frames: u32,
  pub presentation_interval_distribution: Distribution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
  pub count: u64,
  pub minimum_ns: u64,
  pub p50_ns: u64,
  pub p95_ns: u64,
  pub p99_ns: u64,
  pub maximum_ns: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResultCommand {
  Run,
  Capture,
  Profile,
  ComparisonOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
  Passed,
  Failed,
  InfrastructureError,
  Interrupted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuildResult {
  pub source_fingerprint: String,
  pub fingerprint: String,
  pub disposition: BuildDisposition,
  pub duration_ms: u64,
  pub log_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuildDisposition {
  Created,
  Reused,
  RequiredByNoBuild,
  Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseResult {
  pub name: PhaseName,
  pub status: PhaseStatus,
  pub duration_ms: u64,
  pub expired_deadline: Option<DeadlineKind>,
  pub log_path: Option<String>,
  pub error_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseName {
  Discovery,
  Build,
  Launch,
  Startup,
  Scenarios,
  Cleanup,
  SimulatorBoot,
  Reset,
  BaselineDownload,
  Comparison,
  Media,
  Durability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseStatus {
  Passed,
  Failed,
  Interrupted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlayerSessionResult {
  pub player_session_id: String,
  pub accepted: bool,
  pub startup_report: StartupReport,
  pub diagnostic_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobResult {
  pub job_id: String,
  pub player_session_id: String,
  pub status: JobStatus,
  pub first_scenario_index: Option<u32>,
  pub last_scenario_index: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
  Passed,
  Failed,
  InfrastructureError,
  Interrupted,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioResult {
  pub id: String,
  pub name: String,
  pub status: ScenarioStatus,
  pub status_reason: Option<String>,
  pub motion: Motion,
  pub duration_ms: u64,
  pub expired_deadline: Option<DeadlineKind>,
  pub timings: ScenarioTimings,
  pub steps: Vec<StepResult>,
  pub logs: Option<LogSpan>,
  pub failure_frame: Option<MediaCapture>,
  pub recovery: Recovery,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub performance_attempt: Option<crate::wire::job::PerformanceAttempt>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioStatus {
  Passed,
  Failed,
  Skipped,
  NotRun,
  InfrastructureError,
  Interrupted,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTimings {
  pub startup_ms: Option<u64>,
  pub settle_ms: Option<u64>,
  pub capture_ms: Option<u64>,
  pub baseline_read_ms: Option<u64>,
  pub odiff_ms: Option<u64>,
  pub reset_ms: Option<u64>,
  pub baseline_download_ms: Option<u64>,
  pub comparison_ms: Option<u64>,
  pub media_ms: Option<u64>,
  pub durability_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogSpan {
  pub job_id: String,
  pub player_session_id: String,
  pub first_sequence: u64,
  pub last_sequence: u64,
  pub complete: bool,
  pub path: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Recovery {
  None,
  Reset,
  Relaunch,
  RelaunchFailed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StepResult {
  pub index: u32,
  pub name: Option<String>,
  pub kind: StepName,
  pub status: StepStatus,
  pub status_reason: Option<String>,
  pub duration_ms: u64,
  pub expired_deadline: Option<DeadlineKind>,
  pub error_ids: Vec<String>,
  pub assertion: Option<AssertionResult>,
  pub screenshot: Option<ScreenshotResult>,
  pub video: Option<VideoResult>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub performance: Option<StepPerformance>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineWriteResult {
  pub sha256: String,
  pub profile: String,
  pub scenario: String,
  pub checkpoint: String,
  pub status: BaselineWriteStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BaselineWriteStatus {
  Proposed,
  UploadedUnreferenced,
  Published,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)]
pub enum ScreenshotResult {
  Captured {
    checkpoint: String,
    actual: ImageFile,
    baseline: BaselineOutcome,
    comparison: Option<ComparisonOutcome>,
    matched_before_update: Option<bool>,
    updated: Option<bool>,
  },
  Unavailable {
    reason: String,
    error_id: String,
  },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum BaselineOutcome {
  NotLoaded,
  Missing,
  Loaded { image: ImageFile },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ComparisonOutcome {
  Passed {
    changed_pixels: u64,
    total_pixels: u64,
    settings: Comparison,
  },
  Mismatch {
    changed_pixels: u64,
    total_pixels: u64,
    settings: Comparison,
    diff: ImageFile,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImageFile {
  pub path: String,
  pub sha256: String,
  pub width: u32,
  pub height: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum VideoResult {
  Encoded {
    path: String,
    sha256: String,
    width: u32,
    height: u32,
    frame_rate: u32,
    duration_ms: u64,
    truncated: bool,
  },
  Failed {
    error_id: String,
    diagnostic_paths: Vec<String>,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MediaCapture {
  Captured {
    image: ImageFile,
  },
  Unavailable {
    reason: String,
    error_id: Option<String>,
  },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorOccurrence {
  pub id: String,
  pub code: ErrorCode,
  pub source: ErrorSource,
  pub message: String,
  pub job_id: Option<String>,
  pub player_session_id: Option<String>,
  pub scenario_id: Option<String>,
  pub step_index: Option<u32>,
  pub log_sequence: Option<u64>,
}

impl RunResult {
  /// Validates all cross-reference and conditional result invariants.
  pub fn validate(&self) -> Result<()> {
    result_validation::validate_run_result(self)
  }

  /// Serializes the result with lexical keys, two-space indentation, and one newline.
  pub fn to_canonical_json(&self) -> Result<Vec<u8>> {
    self.validate()?;
    result_format::canonical_pretty_json(self)
  }

  /// Serializes the result as one canonical JSON line.
  pub fn to_canonical_json_line(&self) -> Result<Vec<u8>> {
    self.validate()?;
    result_format::canonical_json_line(self)
  }
}

impl<'de> Deserialize<'de> for BaselineOutcome {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Ok(match RawBaselineOutcome::deserialize(deserializer)? {
      RawBaselineOutcome::NotLoaded {} => Self::NotLoaded,
      RawBaselineOutcome::Missing {} => Self::Missing,
      RawBaselineOutcome::Loaded { image } => Self::Loaded { image },
    })
  }
}

#[derive(Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case", deny_unknown_fields)]
enum RawBaselineOutcome {
  NotLoaded {},
  Missing {},
  Loaded { image: ImageFile },
}
