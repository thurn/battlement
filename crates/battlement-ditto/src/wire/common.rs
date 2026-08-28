//! Enums and small values shared across Ditto wire documents.

use serde::{Deserialize, Serialize};

use crate::wire::job::ObjectState;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepName {
  Click,
  Hover,
  Drag,
  Key,
  Wait,
  Assert,
  Screenshot,
  Video,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepStatus {
  Passed,
  Failed,
  NotRun,
  InfrastructureError,
  Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeadlineKind {
  Step,
  Scenario,
  Run,
  Reset,
  BaselineDownload,
  Build,
  Launch,
  Startup,
  SimulatorBoot,
  Comparison,
  Media,
  Durability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ErrorCode {
  #[serde(rename = "configuration.invalid")]
  ConfigurationInvalid,
  #[serde(rename = "build.failed")]
  BuildFailed,
  #[serde(rename = "launch.failed")]
  LaunchFailed,
  #[serde(rename = "simulator.boot-failed")]
  SimulatorBootFailed,
  #[serde(rename = "startup.mismatch")]
  StartupMismatch,
  #[serde(rename = "startup.probe-failed")]
  StartupProbeFailed,
  #[serde(rename = "assertion.failed")]
  AssertionFailed,
  #[serde(rename = "input.unreachable")]
  InputUnreachable,
  #[serde(rename = "condition.unsupported")]
  ConditionUnsupported,
  #[serde(rename = "image.mismatch")]
  ImageMismatch,
  #[serde(rename = "image.missing-baseline")]
  ImageMissingBaseline,
  #[serde(rename = "image.capture-failed")]
  ImageCaptureFailed,
  #[serde(rename = "image.comparison-failed")]
  ImageComparisonFailed,
  #[serde(rename = "baseline.download-failed")]
  BaselineDownloadFailed,
  #[serde(rename = "baseline.hash-mismatch")]
  BaselineHashMismatch,
  #[serde(rename = "baseline.store-conflict")]
  BaselineStoreConflict,
  #[serde(rename = "runtime.unity-error")]
  RuntimeUnityError,
  #[serde(rename = "runtime.unity-assert")]
  RuntimeUnityAssert,
  #[serde(rename = "runtime.unity-exception")]
  RuntimeUnityException,
  #[serde(rename = "runtime.fatal")]
  RuntimeFatal,
  #[serde(rename = "runtime.panic")]
  RuntimePanic,
  #[serde(rename = "runtime.process-exit")]
  RuntimeProcessExit,
  #[serde(rename = "runtime.reset-failed")]
  RuntimeResetFailed,
  #[serde(rename = "runtime.destroy-failed")]
  RuntimeDestroyFailed,
  #[serde(rename = "deadline.expired")]
  DeadlineExpired,
  #[serde(rename = "transport.request-failed")]
  TransportRequestFailed,
  #[serde(rename = "transport.log-buffer-overflow")]
  TransportLogBufferOverflow,
  #[serde(rename = "transport.log-record-oversize")]
  TransportLogRecordOversize,
  #[serde(rename = "transport.log-gap")]
  TransportLogGap,
  #[serde(rename = "transport.log-conflict")]
  TransportLogConflict,
  #[serde(rename = "transport.artifact-conflict")]
  TransportArtifactConflict,
  #[serde(rename = "media.insufficient-space")]
  MediaInsufficientSpace,
  #[serde(rename = "media.recording-failed")]
  MediaRecordingFailed,
  #[serde(rename = "media.ffmpeg-failed")]
  MediaFfmpegFailed,
  #[serde(rename = "durability.failed")]
  DurabilityFailed,
  #[serde(rename = "durability.result-commit-failed")]
  DurabilityResultCommitFailed,
  #[serde(rename = "baseline.lock-stale")]
  BaselineLockStale,
  #[serde(rename = "baseline.manifest-write-failed")]
  BaselineManifestWriteFailed,
  #[serde(rename = "baseline.publish-failed")]
  BaselinePublishFailed,
  #[serde(rename = "baseline.lease-lost")]
  BaselineLeaseLost,
  #[serde(rename = "baseline.cleanup-failed")]
  BaselineCleanupFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorSource {
  Ditto,
  DittoPlayer,
  Unity,
  Rust,
  #[serde(rename = "odiff")]
  ODiff,
  #[serde(rename = "ffmpeg")]
  FFmpeg,
  Filesystem,
  #[serde(rename = "r2")]
  R2,
}

/// The observed result of one assertion condition.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssertionResult {
  pub object: String,
  pub state: ObjectState,
  pub expected: bool,
  pub observed: bool,
  pub passed: bool,
}
