#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Newtonsoft.Json;

namespace Battlement
{
    internal static class DittoLifecycleValidation
    {
        private const int MaximumRequestBytes = 1024 * 1024;
        private static readonly UTF8Encoding StrictUtf8 = new(false, true);

        public static void ValidateStarted(
            DittoStarted started,
            DittoJob job,
            string expectedPlayerSessionId,
            string? acceptedPlayerSessionId
        )
        {
            DittoJobValidation.Validate(job);
            Identifier("player_session_id", expectedPlayerSessionId);
            Require(started.JobId == job.JobId, "started job_id does not own this job");
            Require(started.RunId == job.RunId, "started run_id does not own this job");
            Require(
                started.PlayerSessionId == expectedPlayerSessionId,
                "started player_session_id does not own this route"
            );
            Require(
                started.StartupFailure is null || started.StartupLogFailure is null,
                "startup_failure and startup_log_failure are mutually exclusive"
            );
            Require(
                started.FirstLogSequence.HasValue || started.StartupLogFailure is not null,
                "first_log_sequence may be null only for a startup log failure"
            );
            if (started.StartupFailure is not null)
            {
                FailureValue(started.StartupFailure);
            }
            if (started.StartupLogFailure is not null)
            {
                FailureValue(started.StartupLogFailure);
            }
            switch (started.Identity)
            {
                case DittoStartupIdentity.Report report:
                    Require(
                        acceptedPlayerSessionId is null,
                        "a new player must report startup identity"
                    );
                    StartupReport(report.StartupReport);
                    break;
                case DittoStartupIdentity.Accepted accepted:
                    Identifier("accepted_player_session_id", accepted.AcceptedPlayerSessionId);
                    Require(
                        acceptedPlayerSessionId == accepted.AcceptedPlayerSessionId,
                        "accepted player session does not match the warm route"
                    );
                    Require(
                        accepted.AcceptedPlayerSessionId == started.PlayerSessionId,
                        "accepted identity must equal player_session_id"
                    );
                    break;
                default:
                    throw new JsonSerializationException("Unknown startup identity.");
            }
        }

        public static void ValidateLogAck(
            DittoLogBatchAck ack,
            string playerSessionId,
            ulong expectedNextSequence
        )
        {
            Identifier("player_session_id", ack.PlayerSessionId);
            Require(
                ack.PlayerSessionId == playerSessionId,
                "log acknowledgement belongs to another player session"
            );
            Require(
                ack.NextSequence == expectedNextSequence,
                "log acknowledgement has the wrong next sequence"
            );
        }

        public static void ValidateArtifactAck(
            DittoArtifactAck ack,
            string artifactId,
            string sha256
        )
        {
            Identifier("artifact_id", ack.ArtifactId);
            Sha256("artifact sha256", ack.Sha256);
            Require(
                ack.ArtifactId == artifactId,
                "artifact acknowledgement has the wrong artifact_id"
            );
            Require(ack.Sha256 == sha256, "artifact acknowledgement has the wrong hash");
        }

        public static void ValidateScenarioDecision(DittoScenarioDecision decision)
        {
            bool hasNoError =
                decision.ErrorId is null && decision.ErrorCode is null && decision.Message is null;
            if (decision.Action == DittoNextAction.Continue)
            {
                Require(hasNoError, "continue decision must not contain error fields");
                return;
            }
            Require(
                decision.ErrorId is not null && decision.ErrorCode is not null,
                "stop and relaunch decisions require every error field"
            );
            Require(
                decision.Message is not null,
                "stop and relaunch decisions require every error field"
            );
            HostErrorId(decision.ErrorId!);
            Reason("decision.message", decision.Message!);
        }

        public static void ValidateJobComplete(DittoJobComplete complete, DittoJob job)
        {
            Require(complete.JobId == job.JobId, "completion job_id does not own this job");
            Require(
                complete.ExecutionDurationMs <= job.RemainingRunTimeoutMs,
                "job execution duration exceeds its remaining deadline"
            );
            TerminalAccounting(job, complete.ExecutedScenarioIds, complete.UnstartedScenarios);
            if (complete.Reason == DittoTerminalReason.Completed)
            {
                Require(
                    complete.UnstartedScenarios.Count == 0,
                    "completed job may not contain unstarted scenarios"
                );
            }
        }

        public static void ValidateJobFailed(DittoJobFailed failed, DittoJob job)
        {
            Require(failed.JobId == job.JobId, "failure job_id does not own this job");
            FailureValue(failed.Failure);
            TerminalAccounting(job, failed.ExecutedScenarioIds, failed.UnstartedScenarios);
        }

        public static void ValidateCompleteAck(DittoJobCompleteAck ack, string jobId)
        {
            Identifier("job_id", ack.JobId);
            Require(ack.JobId == jobId, "completion acknowledgement has the wrong job_id");
        }

        public static void ValidateFailedAck(DittoJobFailedAck ack, string jobId)
        {
            ValidateCompleteAck(new DittoJobCompleteAck(ack.JobId), jobId);
            HostErrorId(ack.ErrorId);
        }

        public static void ValidateHttpError(DittoHttpError error)
        {
            HostErrorId(error.ErrorId);
            Reason("http error message", error.Message);
            Require(
                !error.ExpectedSequence.HasValue || error.Code == DittoErrorCode.TransportLogGap,
                "expected_sequence is valid only for transport.log-gap"
            );
            if (error.RelatedRunId is not null)
            {
                Identifier("related_run_id", error.RelatedRunId);
            }
        }

        public static IReadOnlyList<DittoEventRecord> DecodeNdjson(
            ReadOnlyMemory<byte> bytes,
            DittoJob job,
            string playerSessionId,
            ulong firstSequence
        )
        {
            Require(bytes.Length > 0, "NDJSON body must not be empty");
            Require(bytes.Length <= MaximumRequestBytes, "NDJSON body exceeds 1 MiB");
            string source;
            try
            {
                source = StrictUtf8.GetString(bytes.Span);
            }
            catch (DecoderFallbackException exception)
            {
                throw new JsonSerializationException("NDJSON body must be UTF-8", exception);
            }
            Require(
                source.EndsWith("\n", StringComparison.Ordinal),
                "NDJSON body must end with LF"
            );
            source = source[..^1];
            Require(source.Length > 0, "NDJSON body must contain a record");
            string[] lines = source.Split('\n');
            var records = new List<DittoEventRecord>(lines.Length);
            for (var offset = 0; offset < lines.Length; offset++)
            {
                string line = lines[offset];
                Require(line.Length > 0, "NDJSON body must not contain blank lines");
                Require(
                    Encoding.UTF8.GetByteCount(line) < MaximumRequestBytes,
                    "one NDJSON record exceeds 1 MiB"
                );
                ulong sequence;
                try
                {
                    sequence = checked(firstSequence + (ulong)offset);
                }
                catch (OverflowException exception)
                {
                    throw new JsonSerializationException("log sequence overflow", exception);
                }
                DittoEventRecord record = DittoLifecycleCodec.Decode<DittoEventRecord>(
                    Encoding.UTF8.GetBytes(line)
                );
                ValidateEventRecord(record, job, playerSessionId, sequence);
                records.Add(record);
            }
            return records;
        }

        internal static DittoResolvedScenario Scenario(DittoJob job, string scenarioId)
        {
            Identifier("scenario_id", scenarioId);
            return job.Scenarios.FirstOrDefault(scenario => scenario.Id == scenarioId)
                ?? throw new JsonSerializationException("scenario_id does not belong to this job");
        }

        internal static DittoStepName StepName(DittoStepAction action) =>
            action switch
            {
                DittoStepAction.Click => DittoStepName.Click,
                DittoStepAction.Hover => DittoStepName.Hover,
                DittoStepAction.Drag => DittoStepName.Drag,
                DittoStepAction.Key => DittoStepName.Key,
                DittoStepAction.Wait => DittoStepName.Wait,
                DittoStepAction.Assert => DittoStepName.Assert,
                DittoStepAction.Screenshot => DittoStepName.Screenshot,
                DittoStepAction.Video => DittoStepName.Video,
                _ => throw new JsonSerializationException("Unknown job step action."),
            };

        internal static void FailureFrame(DittoPlayerFailureFrame frame)
        {
            switch (frame)
            {
                case DittoPlayerFailureFrame.Captured captured:
                    Identifier("failure frame artifact_id", captured.ArtifactId);
                    break;
                case DittoPlayerFailureFrame.Unavailable unavailable:
                    Reason("failure frame reason", unavailable.Reason);
                    if (unavailable.ErrorRef is not null)
                    {
                        PlayerErrorRef(unavailable.ErrorRef);
                    }
                    break;
                default:
                    throw new JsonSerializationException("Unknown failure frame.");
            }
        }

        internal static void NativeVideo(
            DittoNativeVideoInput input,
            DittoResolvedScenario scenario
        )
        {
            Identifier("video input_id", input.InputId);
            Sha256("video sha256", input.Sha256);
            Require(
                input.Path.Length is > 0 and <= 1024,
                "native video path must contain 1 through 1024 UTF-8 bytes"
            );
            Require(
                input.Width > 0 && input.Height > 0,
                "native video dimensions must be positive"
            );
            Require(input.FrameCount > 0, "native video frame_count must be positive");
            Require(
                input.StartStepIndex < scenario.Steps.Count,
                "native video start_step_index is outside the scenario"
            );
            Require(
                scenario.Steps[(int)input.StartStepIndex].Action
                    is DittoStepAction.Video { Value: DittoVideo.Start },
                "native video must reference a video start step"
            );
        }

        internal static void ArtifactKind(DittoArtifactKind kind)
        {
            if (kind is DittoArtifactKind.Screenshot screenshot)
            {
                Name("artifact checkpoint", screenshot.Checkpoint);
            }
        }

        internal static void PlayerErrorRef(string value) =>
            NumberedReference("player error reference", value, 'P');

        internal static void HostErrorId(string value) =>
            NumberedReference("host error ID", value, 'E');

        internal static void Identifier(string field, string value)
        {
            bool parsed = Guid.TryParseExact(value, "D", out Guid id);
            Require(parsed, $"{field} must be a UUID");
            Require(id != Guid.Empty, $"{field} must not be nil");
            Require(id.ToString("D") == value, $"{field} must use canonical lowercase UUID text");
        }

        internal static void Sha256(string field, string value) =>
            Require(
                value.Length == 64
                    && value.All(character =>
                        character is >= '0' and <= '9' || character is >= 'a' and <= 'f'
                    ),
                $"{field} must contain exactly 64 lowercase hexadecimal digits"
            );

        internal static void Name(string field, string value)
        {
            Require(value.Length > 0, $"{field} must not be empty");
            Require(Bytes(value) <= 128, $"{field} may contain at most 128 UTF-8 bytes");
        }

        internal static void Reason(string field, string value)
        {
            Require(value.Length > 0, $"{field} must not be empty");
            Require(Bytes(value) <= 4096, $"{field} may contain at most 4096 UTF-8 bytes");
        }

        internal static void Require(bool condition, string message)
        {
            if (!condition)
            {
                throw new JsonSerializationException(message);
            }
        }

        private static void StartupReport(DittoStartupReport report)
        {
            Name("capture_adapter", report.CaptureAdapter);
            Name("unity_version", report.UnityVersion);
            Sha256("build_fingerprint", report.BuildFingerprint);
            Sha256("source_fingerprint", report.SourceFingerprint);
            ValidateDisplay(report.Platform, report.Display);
            var unique = new HashSet<DittoCapability>(report.Capabilities);
            Require(
                unique.Count == report.Capabilities.Count,
                "profile capabilities must be unique"
            );
            DittoCapability? unsupported = report.Platform switch
            {
                DittoPlatform.Webgl => DittoCapability.Video,
                DittoPlatform.IosSimulator => DittoCapability.Hover,
                _ => null,
            };
            Require(
                !unsupported.HasValue || !unique.Contains(unsupported.Value),
                "profile contains a capability unsupported by its platform"
            );
        }

        private static void ValidateDisplay(DittoPlatform platform, DittoDisplay display)
        {
            Require(display.Width > 0 && display.Height > 0, "display dimensions must be positive");
            Require(
                double.IsFinite(display.Scale) && display.Scale > 0,
                "display scale must be finite and positive"
            );
            Require(display.SafeArea.Count == 4, "display safe area requires four values");
            uint x = display.SafeArea[0];
            uint y = display.SafeArea[1];
            uint width = display.SafeArea[2];
            uint height = display.SafeArea[3];
            Require(width > 0 && height > 0, "display safe area must be nonempty");
            Require(
                (ulong)x + width <= display.Width && (ulong)y + height <= display.Height,
                "display safe area must fit inside the framebuffer"
            );
            if (platform == DittoPlatform.IosSimulator)
            {
                Require(
                    display.Orientation.HasValue,
                    "iOS Simulator display requires an orientation"
                );
                return;
            }
            Require(!display.Orientation.HasValue, "desktop display must not have an orientation");
            bool origin = x == 0 && y == 0;
            bool dimensions = width == display.Width && height == display.Height;
            Require(origin && dimensions, "desktop safe area must equal the framebuffer");
        }

        private static void FailureValue(DittoPlayerInfrastructureFailure failure) =>
            Reason("infrastructure failure message", failure.Message);

        private static void TerminalAccounting(
            DittoJob job,
            IReadOnlyList<string> executed,
            IReadOnlyList<DittoUnstartedScenario> unstarted
        )
        {
            Require(
                executed.Count + unstarted.Count == job.Scenarios.Count,
                "terminal scenario accounting must cover every job scenario"
            );
            var ids = new HashSet<string>(StringComparer.Ordinal);
            for (var index = 0; index < executed.Count; index++)
            {
                Identifier("executed scenario_id", executed[index]);
                Require(ids.Add(executed[index]), "terminal scenario IDs must be unique");
                Require(
                    job.Scenarios[index].Id == executed[index],
                    "executed scenarios must be an ordered job prefix"
                );
            }
            for (var offset = 0; offset < unstarted.Count; offset++)
            {
                DittoUnstartedScenario entry = unstarted[offset];
                Identifier("unstarted scenario_id", entry.ScenarioId);
                Reason("unstarted scenario reason", entry.Reason);
                Require(ids.Add(entry.ScenarioId), "terminal scenario IDs must be unique");
                Require(
                    job.Scenarios[executed.Count + offset].Id == entry.ScenarioId,
                    "unstarted scenarios must be the ordered job suffix"
                );
            }
        }

        private static void ValidateEventRecord(
            DittoEventRecord record,
            DittoJob job,
            string playerSessionId,
            ulong expectedSequence
        )
        {
            var common = (IDittoEventRecord)record;
            Require(common.Schema == 1, "log schema must equal 1");
            Require(common.JobId == job.JobId, "log record belongs to another job");
            Identifier("player_session_id", common.PlayerSessionId);
            Require(
                common.PlayerSessionId == playerSessionId,
                "log record belongs to another player session"
            );
            Require(
                common.Sequence == expectedSequence,
                "log records must have contiguous sequences"
            );
            Name("event_name", common.EventName);
            Require(
                Bytes(common.Message) <= 4096,
                "log message may contain at most 4096 UTF-8 bytes"
            );
            switch (record)
            {
                case DittoOrdinaryLogRecord log:
                    ValidateLogRecord(log);
                    break;
                case DittoContextRecord context:
                    ValidateContext(context, job);
                    break;
                default:
                    throw new JsonSerializationException("Unknown log record variant.");
            }
        }

        private static void ValidateLogRecord(DittoOrdinaryLogRecord record)
        {
            Require(record.Fields.Count <= 128, "log fields may contain at most 128 entries");
            foreach ((string key, string value) in record.Fields)
            {
                Name("log field name", key);
                Require(
                    Bytes(value) <= 4096,
                    "log field value may contain at most 4096 UTF-8 bytes"
                );
            }
        }

        private static void ValidateContext(DittoContextRecord record, DittoJob job)
        {
            switch (record.Body)
            {
                case DittoContext.JobStarted body:
                    Require(body.RunId == job.RunId, "job-started context has the wrong run_id");
                    break;
                case DittoContext.JobEnded:
                    break;
                case DittoContext.EngineStarted body:
                    Identifier("engine_session_id", body.EngineSessionId);
                    Scenario(job, body.ScenarioId);
                    break;
                case DittoContext.EngineEnded body:
                    Identifier("engine_session_id", body.EngineSessionId);
                    break;
                case DittoContext.ScenarioStarted body:
                    Scenario(job, body.ScenarioId);
                    break;
                case DittoContext.ScenarioEnded body:
                    ValidateScenarioEnded(body, job);
                    break;
                case DittoContext.StepStarted body:
                    Require(
                        body.StepIndex < Scenario(job, body.ScenarioId).Steps.Count,
                        "step-started index is outside the scenario"
                    );
                    break;
                case DittoContext.StepEnded body:
                    DittoResolvedScenario scenario = Scenario(job, body.ScenarioId);
                    Require(
                        body.Result.Index < scenario.Steps.Count,
                        "step-ended index is outside the scenario"
                    );
                    DittoCompletionValidation.ValidateStepResult(
                        scenario.Steps[(int)body.Result.Index],
                        body.Result
                    );
                    break;
                case DittoContext.ArtifactAccepted body:
                    ValidateArtifactContext(body, job);
                    break;
                case DittoContext.ErrorObserved body:
                    ValidateErrorContext(record, body, job);
                    break;
                default:
                    throw new JsonSerializationException("Unknown context variant.");
            }
        }

        private static void ValidateScenarioEnded(DittoContext.ScenarioEnded body, DittoJob job)
        {
            DittoResolvedScenario scenario = Scenario(job, body.ScenarioId);
            if (body.FailureFrame is not null)
            {
                FailureFrame(body.FailureFrame);
            }
            Require(
                body.VideoInputs.Count <= 64,
                "scenario context may contain at most 64 video inputs"
            );
            var ids = new HashSet<string>(StringComparer.Ordinal);
            foreach (DittoNativeVideoInput input in body.VideoInputs)
            {
                NativeVideo(input, scenario);
                Require(ids.Add(input.InputId), "scenario context video IDs must be unique");
            }
            if (body.PrimaryErrorRef is not null)
            {
                PlayerErrorRef(body.PrimaryErrorRef);
            }
            if (body.Boundary is DittoScenarioBoundary.Failed failed)
            {
                PlayerErrorRef(failed.ErrorRef);
            }
        }

        private static void ValidateArtifactContext(
            DittoContext.ArtifactAccepted body,
            DittoJob job
        )
        {
            DittoResolvedScenario scenario = Scenario(job, body.ScenarioId);
            Identifier("artifact_id", body.ArtifactId);
            ArtifactKind(body.ArtifactKind);
            if (body.ArtifactKind is DittoArtifactKind.FailureFrame)
            {
                Require(
                    !body.StepIndex.HasValue || body.StepIndex.Value < scenario.Steps.Count,
                    "failure frame step is outside scenario"
                );
                return;
            }
            Require(body.StepIndex.HasValue, "screenshot artifact context requires step_index");
            Require(
                body.StepIndex!.Value < scenario.Steps.Count,
                "artifact step_index is outside scenario"
            );
            DittoResolvedStep step = scenario.Steps[(int)body.StepIndex.Value];
            Require(
                step.Action is DittoStepAction.Screenshot,
                "screenshot artifact context must reference a screenshot step"
            );
            string checkpoint = ((DittoArtifactKind.Screenshot)body.ArtifactKind).Checkpoint;
            string expected = ((DittoStepAction.Screenshot)step.Action).Value.Name;
            Require(checkpoint == expected, "artifact checkpoint does not match the job");
        }

        private static void ValidateErrorContext(
            DittoContextRecord record,
            DittoContext.ErrorObserved body,
            DittoJob job
        )
        {
            DittoResolvedScenario scenario = Scenario(job, body.ScenarioId);
            Require(
                !body.StepIndex.HasValue || body.StepIndex.Value < scenario.Steps.Count,
                "error step index is outside scenario"
            );
            PlayerErrorRef(body.ErrorRef);
            Require(
                !body.RecordSequence.HasValue || body.RecordSequence.Value < record.Sequence,
                "observed error must follow its source record"
            );
            if (body.BattlementErrorId is not null)
            {
                Name("battlement_error_id", body.BattlementErrorId);
            }
        }

        private static void NumberedReference(string field, string value, char prefix)
        {
            Require(
                value.Length == 5 && value[0] == prefix,
                $"{field} must use {prefix}#### syntax"
            );
            string digits = value[1..];
            Require(
                digits.All(character => character is >= '0' and <= '9') && digits != "0000",
                $"{field} must use a positive four-digit sequence"
            );
        }

        private static int Bytes(string value) => Encoding.UTF8.GetByteCount(value);
    }
}
