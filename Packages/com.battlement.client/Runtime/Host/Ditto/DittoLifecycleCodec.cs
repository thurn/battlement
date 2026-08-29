#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Linq;
using Newtonsoft.Json.Serialization;

namespace Battlement
{
    internal static class DittoLifecycleCodec
    {
        private static readonly UTF8Encoding StrictUtf8 = new(false, true);
        private static readonly JsonSerializerSettings Settings = CreateSettings();

        public static T Decode<T>(ReadOnlyMemory<byte> bytes)
        {
            try
            {
                JToken token = Parse(StrictUtf8.GetString(bytes.Span));
                return token.ToObject<T>(JsonSerializer.Create(Settings))
                    ?? throw new JsonSerializationException("A required Ditto value was null.");
            }
            catch (JsonSerializationException)
            {
                throw;
            }
            catch (Exception exception)
            {
                throw new JsonSerializationException(
                    "The Ditto lifecycle value is invalid: " + exception.Message,
                    exception
                );
            }
        }

        public static byte[] Encode<T>(T value)
        {
            var output = new StringBuilder();
            using var writer = new StringWriter(output, CultureInfo.InvariantCulture);
            using var jsonWriter = new JsonTextWriter(writer);
            JsonSerializer serializer = JsonSerializer.Create(Settings);
            if (DittoLifecycleUnionConverter.IsUnionValue(value))
            {
                new DittoLifecycleUnionConverter().WriteJson(jsonWriter, value, serializer);
            }
            else
            {
                serializer.Serialize(jsonWriter, value, typeof(T));
            }
            jsonWriter.Flush();
            return StrictUtf8.GetBytes(output.ToString());
        }

        public static byte[] EncodeNdjson(IReadOnlyList<DittoEventRecord> records)
        {
            var output = new StringBuilder();
            JsonSerializer serializer = JsonSerializer.Create(Settings);
            foreach (DittoEventRecord record in records)
            {
                using var writer = new StringWriter(output, CultureInfo.InvariantCulture);
                using var jsonWriter = new JsonTextWriter(writer);
                new DittoLifecycleUnionConverter().WriteJson(jsonWriter, record, serializer);
                jsonWriter.Flush();
                output.Append('\n');
            }
            return StrictUtf8.GetBytes(output.ToString());
        }

        public static IReadOnlyList<DittoEventRecord> DecodeNdjson(
            ReadOnlyMemory<byte> bytes,
            DittoJob job,
            string playerSessionId,
            ulong firstSequence
        ) => DittoLifecycleValidation.DecodeNdjson(bytes, job, playerSessionId, firstSequence);

        private static JsonSerializerSettings CreateSettings()
        {
            var settings = new JsonSerializerSettings
            {
                ContractResolver = new DittoLifecycleContractResolver
                {
                    NamingStrategy = new SnakeCaseNamingStrategy(),
                },
                Culture = CultureInfo.InvariantCulture,
                DateParseHandling = DateParseHandling.None,
                DefaultValueHandling = DefaultValueHandling.Include,
                MaxDepth = 128,
                MissingMemberHandling = MissingMemberHandling.Error,
                NullValueHandling = NullValueHandling.Include,
                TypeNameHandling = TypeNameHandling.None,
            };
            settings.Converters.Add(new DittoLifecycleUnionConverter());
            settings.Converters.Add(new DittoLifecycleScalarConverter());
            settings.Converters.Add(new StringEnumConverter(new KebabCaseNamingStrategy(), false));
            return settings;
        }

        private static JToken Parse(string text)
        {
            using var input = new StringReader(text);
            using var reader = new JsonTextReader(input)
            {
                DateParseHandling = DateParseHandling.None,
                MaxDepth = 128,
            };
            JToken token = JToken.ReadFrom(
                reader,
                new JsonLoadSettings
                {
                    DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error,
                }
            );
            if (reader.Read())
            {
                throw new JsonSerializationException("A lifecycle body must contain one value.");
            }
            return token;
        }
    }

    internal sealed class DittoLifecycleContractResolver : DefaultContractResolver
    {
        protected override JsonObjectContract CreateObjectContract(Type objectType)
        {
            JsonObjectContract contract = base.CreateObjectContract(objectType);
            ConstructorInfo? constructor = objectType
                .GetConstructors()
                .OrderByDescending(value => value.GetParameters().Length)
                .FirstOrDefault(value => value.GetParameters().Length > 0);
            if (constructor is null || objectType.IsAbstract)
            {
                return contract;
            }
            contract.CreatorParameters.Clear();
            foreach (ParameterInfo parameter in constructor.GetParameters())
            {
                JsonProperty property =
                    contract.Properties.FirstOrDefault(value =>
                        string.Equals(
                            value.UnderlyingName,
                            parameter.Name,
                            StringComparison.OrdinalIgnoreCase
                        )
                    )
                    ?? throw new JsonSerializationException(
                        $"No lifecycle property matches {objectType.Name}.{parameter.Name}."
                    );
                property.Required = AllowsNull(parameter) ? Required.AllowNull : Required.Always;
                property.NullValueHandling = NullValueHandling.Include;
                contract.CreatorParameters.Add(property);
            }
            contract.OverrideCreator = arguments => constructor.Invoke(arguments);
            return contract;
        }

        private static bool AllowsNull(ParameterInfo parameter) =>
            !parameter.ParameterType.IsValueType
            || Nullable.GetUnderlyingType(parameter.ParameterType) is not null;
    }

    internal sealed class DittoLifecycleScalarConverter : JsonConverter
    {
        private static readonly IReadOnlyDictionary<DittoErrorCode, string> ErrorCodes =
            new Dictionary<DittoErrorCode, string>
            {
                [DittoErrorCode.ConfigurationInvalid] = "configuration.invalid",
                [DittoErrorCode.BuildFailed] = "build.failed",
                [DittoErrorCode.LaunchFailed] = "launch.failed",
                [DittoErrorCode.SimulatorBootFailed] = "simulator.boot-failed",
                [DittoErrorCode.StartupMismatch] = "startup.mismatch",
                [DittoErrorCode.StartupProbeFailed] = "startup.probe-failed",
                [DittoErrorCode.AssertionFailed] = "assertion.failed",
                [DittoErrorCode.InputUnreachable] = "input.unreachable",
                [DittoErrorCode.ConditionUnsupported] = "condition.unsupported",
                [DittoErrorCode.ImageMismatch] = "image.mismatch",
                [DittoErrorCode.ImageMissingBaseline] = "image.missing-baseline",
                [DittoErrorCode.ImageCaptureFailed] = "image.capture-failed",
                [DittoErrorCode.ImageComparisonFailed] = "image.comparison-failed",
                [DittoErrorCode.BaselineDownloadFailed] = "baseline.download-failed",
                [DittoErrorCode.BaselineHashMismatch] = "baseline.hash-mismatch",
                [DittoErrorCode.BaselineStoreConflict] = "baseline.store-conflict",
                [DittoErrorCode.RuntimeUnityError] = "runtime.unity-error",
                [DittoErrorCode.RuntimeUnityAssert] = "runtime.unity-assert",
                [DittoErrorCode.RuntimeUnityException] = "runtime.unity-exception",
                [DittoErrorCode.RuntimeFatal] = "runtime.fatal",
                [DittoErrorCode.RuntimePanic] = "runtime.panic",
                [DittoErrorCode.RuntimeProcessExit] = "runtime.process-exit",
                [DittoErrorCode.RuntimeResetFailed] = "runtime.reset-failed",
                [DittoErrorCode.RuntimeDestroyFailed] = "runtime.destroy-failed",
                [DittoErrorCode.DeadlineExpired] = "deadline.expired",
                [DittoErrorCode.TransportRequestFailed] = "transport.request-failed",
                [DittoErrorCode.TransportLogBufferOverflow] = "transport.log-buffer-overflow",
                [DittoErrorCode.TransportLogRecordOversize] = "transport.log-record-oversize",
                [DittoErrorCode.TransportLogGap] = "transport.log-gap",
                [DittoErrorCode.TransportLogConflict] = "transport.log-conflict",
                [DittoErrorCode.TransportArtifactConflict] = "transport.artifact-conflict",
                [DittoErrorCode.MediaInsufficientSpace] = "media.insufficient-space",
                [DittoErrorCode.MediaRecordingFailed] = "media.recording-failed",
                [DittoErrorCode.MediaFfmpegFailed] = "media.ffmpeg-failed",
                [DittoErrorCode.DurabilityFailed] = "durability.failed",
                [DittoErrorCode.DurabilityResultCommitFailed] = "durability.result-commit-failed",
                [DittoErrorCode.BaselineLockStale] = "baseline.lock-stale",
                [DittoErrorCode.BaselineManifestWriteFailed] = "baseline.manifest-write-failed",
                [DittoErrorCode.BaselinePublishFailed] = "baseline.publish-failed",
                [DittoErrorCode.BaselineLeaseLost] = "baseline.lease-lost",
                [DittoErrorCode.BaselineCleanupFailed] = "baseline.cleanup-failed",
            };

        private static readonly IReadOnlyDictionary<DittoErrorSource, string> ErrorSources =
            new Dictionary<DittoErrorSource, string>
            {
                [DittoErrorSource.Ditto] = "ditto",
                [DittoErrorSource.DittoPlayer] = "ditto-player",
                [DittoErrorSource.Unity] = "unity",
                [DittoErrorSource.Rust] = "rust",
                [DittoErrorSource.ODiff] = "odiff",
                [DittoErrorSource.FFmpeg] = "ffmpeg",
                [DittoErrorSource.Filesystem] = "filesystem",
                [DittoErrorSource.R2] = "r2",
            };

        public override bool CanConvert(Type objectType) =>
            objectType == typeof(DittoErrorCode) || objectType == typeof(DittoErrorSource);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            string value =
                serializer.Deserialize<string>(reader)
                ?? throw new JsonSerializationException("A lifecycle enum may not be null.");
            if (objectType == typeof(DittoErrorCode))
            {
                foreach ((DittoErrorCode code, string wire) in ErrorCodes)
                {
                    if (wire == value)
                    {
                        return code;
                    }
                }
                throw new JsonSerializationException($"Unknown error code {value}.");
            }
            foreach ((DittoErrorSource source, string wire) in ErrorSources)
            {
                if (wire == value)
                {
                    return source;
                }
            }
            throw new JsonSerializationException($"Unknown error source {value}.");
        }

        public override void WriteJson(JsonWriter writer, object? value, JsonSerializer serializer)
        {
            string wire = value switch
            {
                DittoErrorCode code => ErrorCodes[code],
                DittoErrorSource source => ErrorSources[source],
                _ => throw new JsonSerializationException("Unknown lifecycle scalar."),
            };
            writer.WriteValue(wire);
        }
    }

    internal sealed class DittoLifecycleUnionConverter : JsonConverter
    {
        internal static bool IsUnionValue(object? value) =>
            value
                is DittoStartupIdentity
                    or DittoArtifactKind
                    or DittoPlayerFailureFrame
                    or DittoScenarioBoundary
                    or DittoEventRecord
                    or DittoContext;

        public override bool CanConvert(Type objectType) =>
            objectType == typeof(DittoStartupIdentity)
            || objectType == typeof(DittoArtifactKind)
            || objectType == typeof(DittoPlayerFailureFrame)
            || objectType == typeof(DittoScenarioBoundary)
            || objectType == typeof(DittoEventRecord)
            || objectType == typeof(DittoContext);

        public override object ReadJson(
            JsonReader reader,
            Type objectType,
            object? existingValue,
            JsonSerializer serializer
        )
        {
            JObject value = JObject.Load(reader);
            if (objectType == typeof(DittoStartupIdentity))
            {
                return StartupIdentity(value, serializer);
            }
            if (objectType == typeof(DittoArtifactKind))
            {
                return ArtifactKind(value, serializer);
            }
            if (objectType == typeof(DittoPlayerFailureFrame))
            {
                return FailureFrame(value, serializer);
            }
            if (objectType == typeof(DittoScenarioBoundary))
            {
                return Boundary(value, serializer);
            }
            if (objectType == typeof(DittoEventRecord))
            {
                return EventRecord(value, serializer);
            }
            return Context(value, serializer);
        }

        public override void WriteJson(
            JsonWriter writer,
            object? value,
            JsonSerializer serializer
        ) => UnionObject(value, serializer).WriteTo(writer);

        private static DittoStartupIdentity StartupIdentity(
            JObject value,
            JsonSerializer serializer
        )
        {
            if (value.Property("startup_report") is not null)
            {
                Exact(value, "startup_report");
                return new DittoStartupIdentity.Report(
                    Required<DittoStartupReport>(value, "startup_report", serializer)
                );
            }
            Exact(value, "accepted_player_session_id");
            return new DittoStartupIdentity.Accepted(
                RequiredString(value, "accepted_player_session_id")
            );
        }

        private static DittoArtifactKind ArtifactKind(JObject value, JsonSerializer serializer)
        {
            string kind = RequiredString(value, "kind");
            if (kind == "failure-frame")
            {
                Exact(value, "kind");
                return new DittoArtifactKind.FailureFrame();
            }
            if (kind != "screenshot")
            {
                throw new JsonSerializationException($"Unknown artifact kind {kind}.");
            }
            Exact(value, "kind", "checkpoint");
            return new DittoArtifactKind.Screenshot(RequiredString(value, "checkpoint"));
        }

        private static DittoPlayerFailureFrame FailureFrame(
            JObject value,
            JsonSerializer serializer
        )
        {
            string status = RequiredString(value, "status");
            if (status == "captured")
            {
                Exact(value, "status", "artifact_id");
                return new DittoPlayerFailureFrame.Captured(RequiredString(value, "artifact_id"));
            }
            if (status != "unavailable")
            {
                throw new JsonSerializationException($"Unknown failure-frame status {status}.");
            }
            Exact(value, "status", "reason", "error_ref");
            return new DittoPlayerFailureFrame.Unavailable(
                RequiredString(value, "reason"),
                NullableString(value, "error_ref")
            );
        }

        private static DittoScenarioBoundary Boundary(JObject value, JsonSerializer serializer)
        {
            string status = RequiredString(value, "status");
            if (status == "passed")
            {
                Exact(value, "status", "duration_ms");
                return new DittoScenarioBoundary.Passed(
                    Required<ulong>(value, "duration_ms", serializer)
                );
            }
            if (status != "failed")
            {
                throw new JsonSerializationException($"Unknown boundary status {status}.");
            }
            Exact(value, "status", "duration_ms", "stage", "error_ref");
            return new DittoScenarioBoundary.Failed(
                Required<ulong>(value, "duration_ms", serializer),
                Required<DittoBoundaryStage>(value, "stage", serializer),
                RequiredString(value, "error_ref")
            );
        }

        private static DittoEventRecord EventRecord(JObject value, JsonSerializer serializer)
        {
            if (value.Property("body") is not null)
            {
                return value.ToObject<DittoContextRecord>(serializer)!;
            }
            return value.ToObject<DittoOrdinaryLogRecord>(serializer)!;
        }

        private static DittoContext Context(JObject value, JsonSerializer serializer)
        {
            string context = RequiredString(value, "context");
            return context switch
            {
                "job-started" => ContextJobStarted(value),
                "job-ended" => ContextJobEnded(value, serializer),
                "engine-started" => ContextEngineStarted(value),
                "engine-ended" => ContextEngineEnded(value, serializer),
                "scenario-started" => ContextScenarioStarted(value),
                "scenario-ended" => ContextScenarioEnded(value, serializer),
                "step-started" => ContextStepStarted(value, serializer),
                "step-ended" => ContextStepEnded(value, serializer),
                "artifact-accepted" => ContextArtifactAccepted(value, serializer),
                "error-observed" => ContextErrorObserved(value, serializer),
                _ => throw new JsonSerializationException($"Unknown context {context}."),
            };
        }

        private static DittoContext ContextJobStarted(JObject value)
        {
            Exact(value, "context", "run_id");
            return new DittoContext.JobStarted(RequiredString(value, "run_id"));
        }

        private static DittoContext ContextJobEnded(JObject value, JsonSerializer serializer)
        {
            Exact(value, "context", "reason");
            return new DittoContext.JobEnded(
                Required<DittoTerminalReason>(value, "reason", serializer)
            );
        }

        private static DittoContext ContextEngineStarted(JObject value)
        {
            Exact(value, "context", "engine_session_id", "scenario_id");
            return new DittoContext.EngineStarted(
                RequiredString(value, "engine_session_id"),
                RequiredString(value, "scenario_id")
            );
        }

        private static DittoContext ContextEngineEnded(JObject value, JsonSerializer serializer)
        {
            Exact(value, "context", "engine_session_id", "status");
            return new DittoContext.EngineEnded(
                RequiredString(value, "engine_session_id"),
                Required<DittoExecutionStatus>(value, "status", serializer)
            );
        }

        private static DittoContext ContextScenarioStarted(JObject value)
        {
            Exact(value, "context", "scenario_id");
            return new DittoContext.ScenarioStarted(RequiredString(value, "scenario_id"));
        }

        private static DittoContext ContextScenarioEnded(JObject value, JsonSerializer serializer)
        {
            Exact(
                value,
                "context",
                "scenario_id",
                "execution_status",
                "failure_frame",
                "video_inputs",
                "execution_duration_ms",
                "startup_duration_ms",
                "boundary",
                "primary_error_ref"
            );
            return new DittoContext.ScenarioEnded(
                RequiredString(value, "scenario_id"),
                Required<DittoExecutionStatus>(value, "execution_status", serializer),
                NullableClass<DittoPlayerFailureFrame>(value, "failure_frame", serializer),
                Required<IReadOnlyList<DittoNativeVideoInput>>(value, "video_inputs", serializer),
                Required<ulong>(value, "execution_duration_ms", serializer),
                Required<ulong>(value, "startup_duration_ms", serializer),
                Required<DittoScenarioBoundary>(value, "boundary", serializer),
                NullableString(value, "primary_error_ref")
            );
        }

        private static DittoContext ContextStepStarted(JObject value, JsonSerializer serializer)
        {
            Exact(value, "context", "scenario_id", "step_index");
            return new DittoContext.StepStarted(
                RequiredString(value, "scenario_id"),
                Required<uint>(value, "step_index", serializer)
            );
        }

        private static DittoContext ContextStepEnded(JObject value, JsonSerializer serializer)
        {
            Exact(value, "context", "scenario_id", "result");
            return new DittoContext.StepEnded(
                RequiredString(value, "scenario_id"),
                Required<DittoPlayerStepResult>(value, "result", serializer)
            );
        }

        private static DittoContext ContextArtifactAccepted(
            JObject value,
            JsonSerializer serializer
        )
        {
            Exact(value, "context", "scenario_id", "step_index", "artifact_id", "artifact_kind");
            return new DittoContext.ArtifactAccepted(
                RequiredString(value, "scenario_id"),
                NullableValue<uint>(value, "step_index", serializer),
                RequiredString(value, "artifact_id"),
                Required<DittoArtifactKind>(value, "artifact_kind", serializer)
            );
        }

        private static DittoContext ContextErrorObserved(JObject value, JsonSerializer serializer)
        {
            Exact(
                value,
                "context",
                "scenario_id",
                "step_index",
                "error_ref",
                "code",
                "source",
                "record_sequence",
                "battlement_error_id"
            );
            return new DittoContext.ErrorObserved(
                RequiredString(value, "scenario_id"),
                NullableValue<uint>(value, "step_index", serializer),
                RequiredString(value, "error_ref"),
                Required<DittoErrorCode>(value, "code", serializer),
                Required<DittoErrorSource>(value, "source", serializer),
                NullableValue<ulong>(value, "record_sequence", serializer),
                NullableString(value, "battlement_error_id")
            );
        }

        private static JObject UnionObject(object? value, JsonSerializer serializer) =>
            value switch
            {
                DittoStartupIdentity.Report report => Tagged(
                    null,
                    null,
                    serializer,
                    ("startup_report", report.StartupReport)
                ),
                DittoStartupIdentity.Accepted accepted => Tagged(
                    null,
                    null,
                    serializer,
                    ("accepted_player_session_id", accepted.AcceptedPlayerSessionId)
                ),
                DittoArtifactKind.Screenshot screenshot => Tagged(
                    "kind",
                    "screenshot",
                    serializer,
                    ("checkpoint", screenshot.Checkpoint)
                ),
                DittoArtifactKind.FailureFrame => Tagged("kind", "failure-frame", serializer),
                DittoPlayerFailureFrame.Captured captured => Tagged(
                    "status",
                    "captured",
                    serializer,
                    ("artifact_id", captured.ArtifactId)
                ),
                DittoPlayerFailureFrame.Unavailable unavailable => Tagged(
                    "status",
                    "unavailable",
                    serializer,
                    ("reason", unavailable.Reason),
                    ("error_ref", unavailable.ErrorRef)
                ),
                DittoScenarioBoundary.Passed passed => Tagged(
                    "status",
                    "passed",
                    serializer,
                    ("duration_ms", passed.DurationMs)
                ),
                DittoScenarioBoundary.Failed failed => Tagged(
                    "status",
                    "failed",
                    serializer,
                    ("duration_ms", failed.DurationMs),
                    ("stage", failed.Stage),
                    ("error_ref", failed.ErrorRef)
                ),
                DittoContext context => ContextObject(context, serializer),
                DittoContextRecord context => Tagged(
                    null,
                    null,
                    serializer,
                    ("schema", context.Schema),
                    ("job_id", context.JobId),
                    ("player_session_id", context.PlayerSessionId),
                    ("sequence", context.Sequence),
                    ("timestamp_unix_us", context.TimestampUnixUs),
                    ("source", context.Source),
                    ("severity", context.Severity),
                    ("event_name", context.EventName),
                    ("message", context.Message),
                    ("body", context.Body)
                ),
                DittoOrdinaryLogRecord log => Tagged(
                    null,
                    null,
                    serializer,
                    ("schema", log.Schema),
                    ("job_id", log.JobId),
                    ("player_session_id", log.PlayerSessionId),
                    ("sequence", log.Sequence),
                    ("timestamp_unix_us", log.TimestampUnixUs),
                    ("source", log.Source),
                    ("severity", log.Severity),
                    ("event_name", log.EventName),
                    ("message", log.Message),
                    ("fields", log.Fields),
                    ("exception", log.Exception),
                    ("stack_trace", log.StackTrace)
                ),
                _ => throw new JsonSerializationException("Unknown lifecycle union value."),
            };

        private static JObject ContextObject(DittoContext value, JsonSerializer serializer) =>
            value switch
            {
                DittoContext.JobStarted body => Tagged(
                    "context",
                    "job-started",
                    serializer,
                    ("run_id", body.RunId)
                ),
                DittoContext.JobEnded body => Tagged(
                    "context",
                    "job-ended",
                    serializer,
                    ("reason", body.Reason)
                ),
                DittoContext.EngineStarted body => Tagged(
                    "context",
                    "engine-started",
                    serializer,
                    ("engine_session_id", body.EngineSessionId),
                    ("scenario_id", body.ScenarioId)
                ),
                DittoContext.EngineEnded body => Tagged(
                    "context",
                    "engine-ended",
                    serializer,
                    ("engine_session_id", body.EngineSessionId),
                    ("status", body.Status)
                ),
                DittoContext.ScenarioStarted body => Tagged(
                    "context",
                    "scenario-started",
                    serializer,
                    ("scenario_id", body.ScenarioId)
                ),
                DittoContext.ScenarioEnded body => Tagged(
                    "context",
                    "scenario-ended",
                    serializer,
                    ("scenario_id", body.ScenarioId),
                    ("execution_status", body.ExecutionStatus),
                    ("failure_frame", body.FailureFrame),
                    ("video_inputs", body.VideoInputs),
                    ("execution_duration_ms", body.ExecutionDurationMs),
                    ("startup_duration_ms", body.StartupDurationMs),
                    ("boundary", body.Boundary),
                    ("primary_error_ref", body.PrimaryErrorRef)
                ),
                DittoContext.StepStarted body => Tagged(
                    "context",
                    "step-started",
                    serializer,
                    ("scenario_id", body.ScenarioId),
                    ("step_index", body.StepIndex)
                ),
                DittoContext.StepEnded body => Tagged(
                    "context",
                    "step-ended",
                    serializer,
                    ("scenario_id", body.ScenarioId),
                    ("result", body.Result)
                ),
                DittoContext.ArtifactAccepted body => Tagged(
                    "context",
                    "artifact-accepted",
                    serializer,
                    ("scenario_id", body.ScenarioId),
                    ("step_index", body.StepIndex),
                    ("artifact_id", body.ArtifactId),
                    ("artifact_kind", body.ArtifactKind)
                ),
                DittoContext.ErrorObserved body => Tagged(
                    "context",
                    "error-observed",
                    serializer,
                    ("scenario_id", body.ScenarioId),
                    ("step_index", body.StepIndex),
                    ("error_ref", body.ErrorRef),
                    ("code", body.Code),
                    ("source", body.Source),
                    ("record_sequence", body.RecordSequence),
                    ("battlement_error_id", body.BattlementErrorId)
                ),
                _ => throw new JsonSerializationException("Unknown Ditto context."),
            };

        private static JObject Tagged(
            string? tag,
            string? tagValue,
            JsonSerializer serializer,
            params (string Name, object? Value)[] fields
        )
        {
            var output = new JObject();
            if (tag is not null)
            {
                output.Add(tag, tagValue);
            }
            foreach ((string name, object? fieldValue) in fields)
            {
                output.Add(
                    name,
                    fieldValue is null ? JValue.CreateNull()
                        : IsUnionValue(fieldValue) ? UnionObject(fieldValue, serializer)
                        : JToken.FromObject(fieldValue, serializer)
                );
            }
            return output;
        }

        private static T Required<T>(JObject value, string field, JsonSerializer serializer)
        {
            JToken? token = value[field];
            if (token is null)
            {
                throw new JsonSerializationException($"Missing required field {field}.");
            }

            return token.ToObject<T>(serializer)!;
        }

        private static T? NullableClass<T>(JObject value, string field, JsonSerializer serializer)
            where T : class =>
            value[field]?.Type == JTokenType.Null ? null : Required<T>(value, field, serializer);

        private static T? NullableValue<T>(JObject value, string field, JsonSerializer serializer)
            where T : struct =>
            value[field]?.Type == JTokenType.Null ? null : Required<T>(value, field, serializer);

        private static string RequiredString(JObject value, string field) =>
            value[field]?.Type == JTokenType.String
                ? value[field]!.Value<string>()!
                : throw new JsonSerializationException($"{field} must be a string.");

        private static string? NullableString(JObject value, string field) =>
            value[field]?.Type == JTokenType.Null ? null : RequiredString(value, field);

        private static void Exact(JObject value, params string[] fields)
        {
            var expected = new HashSet<string>(fields, StringComparer.Ordinal);
            string? unknown = value
                .Properties()
                .Select(property => property.Name)
                .FirstOrDefault(name => !expected.Contains(name));
            if (unknown is not null)
            {
                throw new JsonSerializationException($"Unknown field {unknown}.");
            }
            string? missing = fields.FirstOrDefault(field => value.Property(field) is null);
            if (missing is not null)
            {
                throw new JsonSerializationException($"Missing required field {missing}.");
            }
        }
    }
}
