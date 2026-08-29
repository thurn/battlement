#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    internal enum DittoExecutionStatus
    {
        Passed,
        Failed,
        Interrupted,
    }

    internal enum DittoBoundaryStage
    {
        Destroy,
        Reset,
    }

    internal enum DittoNextAction
    {
        Continue,
        Stop,
        Relaunch,
    }

    internal enum DittoTerminalReason
    {
        Completed,
        Bail,
        InfrastructureError,
        Interrupted,
    }

    internal enum DittoLogSource
    {
        Battlement,
        Rust,
        Unity,
        DittoPlayer,
    }

    internal enum DittoLogSeverity
    {
        Trace,
        Debug,
        Information,
        Warning,
        Error,
    }

    internal enum DittoStepName
    {
        Click,
        Hover,
        Drag,
        Key,
        Wait,
        Assert,
        Screenshot,
        Video,
    }

    internal enum DittoStepStatus
    {
        Passed,
        Failed,
        NotRun,
        InfrastructureError,
        Interrupted,
    }

    internal enum DittoDeadlineKind
    {
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

    internal enum DittoErrorCode
    {
        ConfigurationInvalid,
        BuildFailed,
        LaunchFailed,
        SimulatorBootFailed,
        StartupMismatch,
        StartupProbeFailed,
        AssertionFailed,
        InputUnreachable,
        ConditionUnsupported,
        ImageMismatch,
        ImageMissingBaseline,
        ImageCaptureFailed,
        ImageComparisonFailed,
        BaselineDownloadFailed,
        BaselineHashMismatch,
        BaselineStoreConflict,
        RuntimeUnityError,
        RuntimeUnityAssert,
        RuntimeUnityException,
        RuntimeFatal,
        RuntimePanic,
        RuntimeProcessExit,
        RuntimeResetFailed,
        RuntimeDestroyFailed,
        DeadlineExpired,
        TransportRequestFailed,
        TransportLogBufferOverflow,
        TransportLogRecordOversize,
        TransportLogGap,
        TransportLogConflict,
        TransportArtifactConflict,
        MediaInsufficientSpace,
        MediaRecordingFailed,
        MediaFfmpegFailed,
        DurabilityFailed,
        DurabilityResultCommitFailed,
        BaselineLockStale,
        BaselineManifestWriteFailed,
        BaselinePublishFailed,
        BaselineLeaseLost,
        BaselineCleanupFailed,
    }

    internal enum DittoErrorSource
    {
        Ditto,
        DittoPlayer,
        Unity,
        Rust,
        ODiff,
        FFmpeg,
        Filesystem,
        R2,
    }

    internal sealed record DittoStarted(
        string JobId,
        string RunId,
        string PlayerSessionId,
        ulong? FirstLogSequence,
        DittoPlayerInfrastructureFailure? StartupFailure,
        DittoPlayerInfrastructureFailure? StartupLogFailure,
        DittoStartupIdentity Identity
    );

    internal abstract record DittoStartupIdentity
    {
        internal sealed record Report(DittoStartupReport StartupReport) : DittoStartupIdentity;

        internal sealed record Accepted(string AcceptedPlayerSessionId) : DittoStartupIdentity;
    }

    internal sealed record DittoStartupReport(
        DittoPlatform Platform,
        string CaptureAdapter,
        string BuildFingerprint,
        string SourceFingerprint,
        string UnityVersion,
        bool Diagnostics,
        DittoDisplay Display,
        IReadOnlyList<DittoCapability> Capabilities
    );

    internal sealed record DittoLogBatchAck(string PlayerSessionId, ulong NextSequence);

    internal sealed record DittoPlayerInfrastructureFailure(DittoErrorCode Code, string Message);

    internal sealed record DittoArtifactAck(string ArtifactId, string Sha256);

    internal sealed record DittoScenarioComplete(
        string ScenarioId,
        DittoExecutionStatus ExecutionStatus,
        IReadOnlyList<DittoPlayerStepResult> Steps,
        IReadOnlyList<DittoReachedArtifact> Artifacts,
        DittoPlayerFailureFrame? FailureFrame,
        IReadOnlyList<DittoNativeVideoInput> VideoInputs,
        ulong LastLogSequence,
        ulong ExecutionDurationMs,
        ulong StartupDurationMs,
        DittoScenarioBoundary Boundary,
        string? PrimaryErrorRef
    );

    internal sealed record DittoPlayerStepResult(
        uint Index,
        string? Name,
        DittoStepName Kind,
        DittoStepStatus Status,
        ulong DurationMs,
        DittoDeadlineKind? ExpiredDeadline,
        IReadOnlyList<string> ErrorRefs,
        DittoAssertionResult? Assertion,
        string? ScreenshotArtifactId,
        string? VideoInputId
    );

    internal sealed record DittoAssertionResult(
        string Object,
        DittoObjectState State,
        bool Expected,
        bool Observed,
        bool Passed
    );

    internal sealed record DittoReachedArtifact(
        string ArtifactId,
        uint? StepIndex,
        DittoArtifactKind Kind
    );

    internal abstract record DittoArtifactKind
    {
        internal sealed record Screenshot(string Checkpoint) : DittoArtifactKind;

        internal sealed record FailureFrame : DittoArtifactKind;
    }

    internal abstract record DittoPlayerFailureFrame
    {
        internal sealed record Captured(string ArtifactId) : DittoPlayerFailureFrame;

        internal sealed record Unavailable(string Reason, string? ErrorRef)
            : DittoPlayerFailureFrame;
    }

    internal sealed record DittoNativeVideoInput(
        string InputId,
        uint StartStepIndex,
        string Path,
        string Sha256,
        uint Width,
        uint Height,
        ulong FrameCount,
        bool Truncated
    );

    internal abstract record DittoScenarioBoundary
    {
        internal sealed record Passed(ulong DurationMs) : DittoScenarioBoundary;

        internal sealed record Failed(ulong DurationMs, DittoBoundaryStage Stage, string ErrorRef)
            : DittoScenarioBoundary;
    }

    internal sealed record DittoScenarioDecision(
        DittoNextAction Action,
        uint CompletedFailures,
        string? ErrorId,
        DittoErrorCode? ErrorCode,
        string? Message
    );

    internal sealed record DittoJobComplete(
        string JobId,
        ulong LastLogSequence,
        IReadOnlyList<string> ExecutedScenarioIds,
        IReadOnlyList<DittoUnstartedScenario> UnstartedScenarios,
        DittoTerminalReason Reason,
        ulong ExecutionDurationMs
    );

    internal sealed record DittoJobCompleteAck(string JobId);

    internal sealed record DittoJobFailed(
        string JobId,
        DittoPlayerInfrastructureFailure Failure,
        ulong? LastLogSequence,
        IReadOnlyList<string> ExecutedScenarioIds,
        IReadOnlyList<DittoUnstartedScenario> UnstartedScenarios
    );

    internal sealed record DittoJobFailedAck(string JobId, string ErrorId);

    internal sealed record DittoUnstartedScenario(string ScenarioId, string Reason);

    internal interface IDittoEventRecord
    {
        uint Schema { get; }
        string JobId { get; }
        string PlayerSessionId { get; }
        ulong Sequence { get; }
        long TimestampUnixUs { get; }
        DittoLogSource Source { get; }
        DittoLogSeverity Severity { get; }
        string EventName { get; }
        string Message { get; }
    }

    internal abstract record DittoEventRecord;

    internal sealed record DittoContextRecord(
        uint Schema,
        string JobId,
        string PlayerSessionId,
        ulong Sequence,
        long TimestampUnixUs,
        DittoLogSource Source,
        DittoLogSeverity Severity,
        string EventName,
        string Message,
        DittoContext Body
    ) : DittoEventRecord, IDittoEventRecord;

    internal sealed record DittoOrdinaryLogRecord(
        uint Schema,
        string JobId,
        string PlayerSessionId,
        ulong Sequence,
        long TimestampUnixUs,
        DittoLogSource Source,
        DittoLogSeverity Severity,
        string EventName,
        string Message,
        IReadOnlyDictionary<string, string> Fields,
        string? Exception,
        string? StackTrace
    ) : DittoEventRecord, IDittoEventRecord;

    internal abstract record DittoContext
    {
        internal sealed record JobStarted(string RunId) : DittoContext;

        internal sealed record JobEnded(DittoTerminalReason Reason) : DittoContext;

        internal sealed record EngineStarted(string EngineSessionId, string ScenarioId)
            : DittoContext;

        internal sealed record EngineEnded(string EngineSessionId, DittoExecutionStatus Status)
            : DittoContext;

        internal sealed record ScenarioStarted(string ScenarioId) : DittoContext;

        internal sealed record ScenarioEnded(
            string ScenarioId,
            DittoExecutionStatus ExecutionStatus,
            DittoPlayerFailureFrame? FailureFrame,
            IReadOnlyList<DittoNativeVideoInput> VideoInputs,
            ulong ExecutionDurationMs,
            ulong StartupDurationMs,
            DittoScenarioBoundary Boundary,
            string? PrimaryErrorRef
        ) : DittoContext;

        internal sealed record StepStarted(string ScenarioId, uint StepIndex) : DittoContext;

        internal sealed record StepEnded(string ScenarioId, DittoPlayerStepResult Result)
            : DittoContext;

        internal sealed record ArtifactAccepted(
            string ScenarioId,
            uint? StepIndex,
            string ArtifactId,
            DittoArtifactKind ArtifactKind
        ) : DittoContext;

        internal sealed record ErrorObserved(
            string ScenarioId,
            uint? StepIndex,
            string ErrorRef,
            DittoErrorCode Code,
            DittoErrorSource Source,
            ulong? RecordSequence,
            string? BattlementErrorId
        ) : DittoContext;
    }

    internal sealed record DittoHttpError(
        string ErrorId,
        DittoErrorCode Code,
        string Message,
        ulong? ExpectedSequence,
        string? RelatedRunId
    );
}
