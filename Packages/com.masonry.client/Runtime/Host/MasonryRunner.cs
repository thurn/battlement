#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using UnityEngine;

namespace Masonry
{
    /// <summary>Scene-authored host for one Masonry session.</summary>
    [DisallowMultipleComponent]
    public sealed class MasonryRunner : MonoBehaviour, IDisposable
    {
        [SerializeField]
        private MasonryTransportKind transportKind;

        [SerializeField]
        private MasonryNativeTransportConfiguration nativeTransport = new();

        [SerializeField]
        private MasonryHttpTransportConfiguration httpTransport = new();

        private MasonryRunnerOptions? options;
        private MasonryWorld? world;
        private MasonryPreparedAssets? preparedAssets;
        private MasonryScenes? scenes;
        private readonly Queue<PendingResponse> pendingResponses = new();
        private PendingSnapshot? pendingSnapshot;
        private TimeSpan? previousStepTime;
        private RunnerState state;
        private SessionId? lastSession;
        private string? pendingConnectionEvent;
        private string? pendingConnectionMessage;
        private bool inputDisabled = true;
        private bool isProcessingResponses;
        private bool wasPaused;
        private bool isDisposed;

        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;
        private const int MaximumDiagnosticBytes = 65_536;
        private static readonly TimeSpan SlowFrameThreshold = TimeSpan.FromMilliseconds(16.67);

        private enum RunnerState
        {
            Stopped,
            AwaitingSnapshot,
            ApplyingSnapshot,
            Running,
        }

        public MasonryTransportKind TransportKind => transportKind;

        public MasonryNativeTransportConfiguration NativeTransport => nativeTransport;

        public MasonryHttpTransportConfiguration HttpTransport => httpTransport;

        /// <summary>Whether Masonry may currently emit pointer and keyboard input.</summary>
        public bool IsInputAvailable => state == RunnerState.Running && !inputDisabled;

        /// <summary>Injects the dependencies owned by this runner.</summary>
        public void Configure(MasonryRunnerOptions runnerOptions)
        {
            MasonryRunnerOptions checkedOptions = Errors.CheckNotNull(
                runnerOptions,
                nameof(runnerOptions)
            );

            if (options is not null)
            {
                throw new InvalidOperationException("The runner is already configured.");
            }

            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryRunner));
            }

            options = checkedOptions;
            world = new MasonryWorld(this);
            preparedAssets = new MasonryPreparedAssets(checkedOptions.AssetStorage);
            scenes = new MasonryScenes(checkedOptions.AssetStorage, preparedAssets, world);
        }

        /// <summary>Starts the configured host session.</summary>
        public void Connect()
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (state != RunnerState.Stopped)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            StartSession(configured, false);
        }

        /// <summary>Stops the current session and starts a new one on the same transport.</summary>
        public void Reconnect()
        {
            MasonryRunnerOptions configured = RequireOptions();
            SessionId? previousSession = lastSession;
            if (state != RunnerState.Stopped)
            {
                StopSession(configured, false);
            }

            StartSession(configured, true, previousSession);
        }

        /// <summary>Stops the active session. Repeated calls are no-ops.</summary>
        public void Stop()
        {
            if (state == RunnerState.Stopped || options is null)
            {
                return;
            }

            StopSession(options, true);
        }

        /// <summary>Submits one already encoded client message to the active session.</summary>
        public void Submit(ReadOnlyMemory<byte> messagePack)
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (state != RunnerState.Running)
            {
                throw new InvalidOperationException(
                    "Client messages may only be submitted while the runner is active."
                );
            }

            TimeSpan started = configured.Clock.Elapsed;
            try
            {
                MasonryTransportResult result;
                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(messagePack);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Submit failed.",
                    duration: configured.Clock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Submit response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started,
                    payloadBytes: messagePack.Length
                );
            }
        }

        /// <summary>Reports a recoverable core failure that stopped one batch.</summary>
        public void ReportBatchFailure(BatchFailed<CoreErrorCode> failure)
        {
            Errors.CheckNotNull(failure, nameof(failure));
            MasonryRunnerOptions configured = RequireRunningSession();
            BatchFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeBatchFailure(bounded),
                "masonry.batch.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode
            );
            ThrowReportedFailureInEditor(bounded.Message);
        }

        /// <summary>Reports a late failure from one nonblocking core operation.</summary>
        public void ReportOperationFailure(OperationFailed<CoreErrorCode> failure)
        {
            Errors.CheckNotNull(failure, nameof(failure));
            MasonryRunnerOptions configured = RequireRunningSession();
            OperationFailed<CoreErrorCode> bounded = failure with
            {
                Message = BoundDiagnostic(failure.Message),
            };
            SubmitFailure(
                configured,
                () => configured.ProtocolCodec.SerializeOperationFailure(bounded),
                "masonry.operation.failed",
                bounded.Message,
                bounded.SessionId,
                bounded.BatchId,
                bounded.CommandId,
                bounded.ErrorCode
            );
            ThrowReportedFailureInEditor(bounded.Message);
        }

        /// <summary>Looks up an asset without starting an implicit load.</summary>
        public bool TryGetPreparedAsset(PreparedAsset asset, out object? value) =>
            preparedAssets?.TryGet(asset, out value) ?? ReturnMissing(out value);

        /// <summary>
        /// Acquires a usage lease that must be disposed when the asset is no longer referenced.
        /// </summary>
        public IMasonryAssetLease AcquirePreparedAsset(PreparedAsset asset) =>
            preparedAssets?.Acquire(asset)
            ?? throw new InvalidOperationException("The runner is not configured.");

        /// <summary>Stops the session and releases the runner's injected dependencies.</summary>
        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            Stop();
            try
            {
                scenes?.Dispose();
            }
            finally
            {
                try
                {
                    preparedAssets?.Dispose();
                }
                finally
                {
                    try
                    {
                        options?.AssetStorage.Dispose();
                    }
                    finally
                    {
                        try
                        {
                            options?.Transport.Dispose();
                        }
                        finally
                        {
                            world?.Dispose();
                            isDisposed = true;
                        }
                    }
                }
            }
        }

        /// <summary>Advances Masonry work for the current Unity frame.</summary>
        public void RunFrame()
        {
            if (state == RunnerState.Stopped || options is null)
            {
                return;
            }

            MasonryRunnerOptions configured = options;
            AdvanceSnapshotPreparation(configured);
            if (state == RunnerState.Stopped)
            {
                return;
            }

            TimeSpan started = configured.Clock.Elapsed;
            TimeSpan previous = previousStepTime ?? started;
            if (started < previous)
            {
                throw new InvalidOperationException("The Masonry clock must be monotonic.");
            }

            int payloadBytes = 0;
            using (MasonryProfiler.Frame.Auto())
            {
                try
                {
                    MasonryTransportResult result;
                    TimeSpan pollStarted = configured.Clock.Elapsed;
                    using (MasonryProfiler.Poll.Auto())
                    using (MasonryProfiler.Transport.Auto())
                    {
                        result = configured.Transport.Poll();
                    }

                    payloadBytes = result.Payload.Length;
                    if (result.Status != MasonryTransportStatus.NoMessage)
                    {
                        ProcessTransportResult(
                            configured,
                            result,
                            "Poll failed.",
                            duration: configured.Clock.Elapsed - pollStarted
                        );
                    }
                }
                catch (Exception exception)
                {
                    FailSession(
                        configured,
                        $"Poll response failed: {exception.Message}",
                        duration: configured.Clock.Elapsed - started,
                        payloadBytes: payloadBytes
                    );
                }
            }

            TimeSpan finished = configured.Clock.Elapsed;
            previousStepTime = finished;
            TimeSpan frameDuration = finished - previous;
            if (frameDuration > SlowFrameThreshold)
            {
                LogSlowFrame(configured, frameDuration, finished - started, payloadBytes);
            }
        }

        private void Update() => RunFrame();

        private void OnApplicationPause(bool pauseStatus)
        {
            if (pauseStatus)
            {
                wasPaused = true;
            }
            else if (wasPaused)
            {
                wasPaused = false;
                Stop();
            }
        }

        private void OnApplicationFocus(bool hasFocus)
        {
            if (!hasFocus && state != RunnerState.Stopped)
            {
                Log(
                    MasonryLogSeverity.Information,
                    "masonry.input.pointer_presses_cancelled",
                    "Pointer presses were cancelled after application focus loss."
                );
            }
        }

        private void OnApplicationQuit() => Stop();

        private void OnDestroy() => Dispose();

        private void StartSession(
            MasonryRunnerOptions configured,
            bool reconnecting,
            SessionId? previousSession = null
        )
        {
            state = RunnerState.AwaitingSnapshot;
            scenes?.BeginSession();
            world?.BeginSession();
            inputDisabled = true;
            previousStepTime = configured.Clock.Elapsed;
            pendingConnectionEvent = reconnecting
                ? "masonry.host.reconnected"
                : "masonry.host.connected";
            pendingConnectionMessage = reconnecting ? "Host reconnected." : "Host connected.";

            TimeSpan started = configured.Clock.Elapsed;
            try
            {
                Connect connect = BuildConnect(configured);
                byte[] bytes;
                using (MasonryProfiler.Serialization.Auto())
                {
                    bytes = configured.ProtocolCodec.SerializeConnect(connect);
                }

                MasonryTransportResult result;
                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Connect(bytes);
                }

                if (result.Status != MasonryTransportStatus.Success)
                {
                    FailSession(
                        configured,
                        "Connect failed.",
                        result,
                        configured.Clock.Elapsed - started
                    );
                    return;
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Connect failed.",
                    true,
                    previousSession,
                    configured.Clock.Elapsed - started
                );
                if (state == RunnerState.Running)
                {
                    LogPendingConnection();
                }
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Connect response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started
                );
            }
        }

        private void ProcessTransportResult(
            MasonryRunnerOptions configured,
            MasonryTransportResult result,
            string failureMessage,
            bool isInitial = false,
            SessionId? previousSession = null,
            TimeSpan? duration = null
        )
        {
            if (result.Status != MasonryTransportStatus.Success)
            {
                FailSession(configured, failureMessage, result, duration);
                return;
            }

            bool outermost = !isProcessingResponses;
            if (outermost)
            {
                isProcessingResponses = true;
            }

            try
            {
                if (result.Payload.Length > MaximumResponseBytes)
                {
                    throw new InvalidDataException(
                        $"A Masonry response cannot exceed {MaximumResponseBytes} bytes."
                    );
                }

                Response response;
                using (MasonryProfiler.ResponseParsing.Auto())
                {
                    response = configured.ProtocolCodec.DeserializeResponse(result.Payload);
                }

                var pending = new PendingResponse(response, isInitial, previousSession);
                if (!outermost)
                {
                    if (pendingResponses.Count >= MaximumQueuedResponses)
                    {
                        throw new InvalidDataException(
                            "Masonry cannot queue more than "
                                + $"{MaximumQueuedResponses} reentrant responses."
                        );
                    }

                    pendingResponses.Enqueue(pending);
                    return;
                }

                if (state == RunnerState.Stopped)
                {
                    return;
                }

                using (MasonryProfiler.ResponseApplication.Auto())
                {
                    ApplyResponse(configured, pending);
                    while (state != RunnerState.Stopped && pendingResponses.Count > 0)
                    {
                        ApplyResponse(configured, pendingResponses.Dequeue());
                    }
                }
            }
            finally
            {
                if (outermost)
                {
                    isProcessingResponses = false;
                    if (state == RunnerState.Stopped)
                    {
                        pendingResponses.Clear();
                    }
                }
            }
        }

        private static Connect BuildConnect(MasonryRunnerOptions configured)
        {
            bool native = configured.Transport.Kind == MasonryTransportKind.Native;
            return new Connect(
                PlatformName(Application.platform),
                Application.unityVersion,
                new ScreenSize(checked((uint)Screen.width), checked((uint)Screen.height)),
                configured.CustomCommandTypes,
                native ? Path.GetFullPath(Application.persistentDataPath) : null,
                native ? Path.GetFullPath(Application.streamingAssetsPath) : null
            );
        }

        private void ApplyResponse(MasonryRunnerOptions configured, PendingResponse pending)
        {
            Response response = pending.Response;
            SessionId? previousSession = pending.PreviousSession;
            if (previousSession is not null && response.SessionId == previousSession.Value)
            {
                Log(
                    MasonryLogSeverity.Warning,
                    "masonry.response.wrong_session",
                    "Discarded a response from the previous session."
                );
                return;
            }

            if (pending.IsInitial)
            {
                if (
                    response.Messages.Count == 0
                    || response.Messages[0] is not ResponseMessage<Command>.SnapshotMessage
                )
                {
                    FailSession(
                        configured,
                        "The first current-session message was not a snapshot."
                    );
                    return;
                }
            }

            if (
                !pending.IsInitial
                && lastSession is not null
                && response.SessionId != lastSession.Value
            )
            {
                Log(
                    MasonryLogSeverity.Warning,
                    "masonry.response.wrong_session",
                    "Discarded a response from a different session."
                );
                return;
            }

            foreach (ResponseMessage<Command> message in response.Messages)
            {
                if (message is ResponseMessage<Command>.SnapshotMessage snapshotMessage)
                {
                    ApplySnapshot(configured, response.SessionId, snapshotMessage.Snapshot);
                    if (state == RunnerState.Stopped)
                    {
                        return;
                    }
                }

                // Batch execution is introduced by Tasks 21-31. Response processing still
                // preserves its position relative to snapshots and later queued returns.
            }
        }

        private void ApplySnapshot(
            MasonryRunnerOptions configured,
            SessionId responseSession,
            Snapshot snapshot
        )
        {
            if (snapshot.SessionId != responseSession)
            {
                FailSession(configured, "A snapshot used the wrong session.");
                return;
            }

            bool isInitialSnapshot = state == RunnerState.AwaitingSnapshot;
            lastSession = responseSession;
            state = RunnerState.ApplyingSnapshot;
            try
            {
                preparedAssets!.BeginReplacement(snapshot.PreparedAssets, isAuthoritative: true);
                pendingSnapshot = new PendingSnapshot(snapshot, isInitialSnapshot);
                AdvanceSnapshotPreparation(configured);
            }
            catch (Exception exception)
            {
                FailSession(configured, $"Snapshot asset preparation failed: {exception.Message}");
            }
        }

        private void AdvanceSnapshotPreparation(MasonryRunnerOptions configured)
        {
            if (state != RunnerState.ApplyingSnapshot || pendingSnapshot is null)
            {
                return;
            }

            if (!pendingSnapshot.SceneReplacementStarted)
            {
                if (!preparedAssets!.TryCompleteReplacement(out MasonryAssetException? error))
                {
                    return;
                }

                if (error is not null)
                {
                    FailSession(configured, $"Snapshot asset preparation failed: {error.Message}");
                    return;
                }

                try
                {
                    scenes!.BeginReplacement(
                        pendingSnapshot.Snapshot.Scenes,
                        pendingSnapshot.Snapshot.PrimarySceneId
                    );
                    pendingSnapshot.SceneReplacementStarted = true;
                }
                catch (Exception exception)
                {
                    FailSession(configured, $"Snapshot scene loading failed: {exception.Message}");
                    return;
                }
            }

            if (!scenes!.TryCompleteReplacement(out MasonryAssetException? sceneError))
            {
                return;
            }

            if (sceneError is not null)
            {
                FailSession(configured, $"Snapshot scene loading failed: {sceneError.Message}");
                return;
            }

            PendingSnapshot completed = pendingSnapshot;
            pendingSnapshot = null;
            try
            {
                if (completed.IsInitial)
                {
                    world?.CreateInitialObjects(completed.Snapshot.Objects);
                }

                inputDisabled = completed.Snapshot.IsInputDisabled;
                state = RunnerState.Running;
                LogPendingConnection();
            }
            catch (Exception exception)
            {
                FailSession(configured, $"Snapshot application failed: {exception.Message}");
            }
        }

        private void FailSession(
            MasonryRunnerOptions configured,
            string message,
            MasonryTransportResult? result = null,
            TimeSpan? duration = null,
            int? payloadBytes = null
        )
        {
            var fields = new Dictionary<string, string>();
            AddSessionField(fields);
            if (duration is not null)
            {
                fields["duration_ms"] = Milliseconds(duration.Value);
            }

            if (result is not null)
            {
                fields["status"] = result.Status.ToString();
                fields["payload_bytes"] = result.Payload.Length.ToString(
                    CultureInfo.InvariantCulture
                );
                if (!string.IsNullOrEmpty(result.Diagnostic))
                {
                    fields["diagnostic"] = result.Diagnostic!;
                }
            }
            else if (payloadBytes is not null)
            {
                fields["payload_bytes"] = payloadBytes.Value.ToString(CultureInfo.InvariantCulture);
            }

            Log(MasonryLogSeverity.Error, "masonry.session.failed", message, fields);
            StopSession(configured, false);
        }

        private void SubmitFailure(
            MasonryRunnerOptions configured,
            Func<byte[]> serialize,
            string eventName,
            string message,
            SessionId sessionId,
            BatchId batchId,
            CommandId? commandId,
            CoreErrorCode errorCode
        )
        {
            var fields = new Dictionary<string, string>
            {
                ["batch_id"] = batchId.Value.ToString(),
                ["error_code"] = errorCode.ToString(),
                ["session_id"] = sessionId.Value.ToString(),
            };
            if (commandId is not null)
            {
                fields["command_id"] = commandId.Value.Value.ToString();
            }

            Log(MasonryLogSeverity.Error, eventName, message, fields);

            TimeSpan started = configured.Clock.Elapsed;
            int payloadBytes = 0;
            try
            {
                MasonryTransportResult result;
                byte[] messagePack;
                using (MasonryProfiler.Serialization.Auto())
                {
                    messagePack = serialize();
                    payloadBytes = messagePack.Length;
                }

                using (MasonryProfiler.Transport.Auto())
                {
                    result = configured.Transport.Submit(messagePack);
                }

                ProcessTransportResult(
                    configured,
                    result,
                    "Failure submission failed.",
                    duration: configured.Clock.Elapsed - started
                );
            }
            catch (Exception exception)
            {
                FailSession(
                    configured,
                    $"Failure submission response failed: {exception.Message}",
                    duration: configured.Clock.Elapsed - started,
                    payloadBytes: payloadBytes
                );
            }
        }

        private static string BoundDiagnostic(string? message)
        {
            if (string.IsNullOrEmpty(message))
            {
                return string.Empty;
            }

            if (Encoding.UTF8.GetByteCount(message) <= MaximumDiagnosticBytes)
            {
                return message;
            }

            int low = 0;
            int high = Math.Min(message.Length, MaximumDiagnosticBytes);
            while (low < high)
            {
                int candidate = low + ((high - low + 1) / 2);
                if (Encoding.UTF8.GetByteCount(message, 0, candidate) <= MaximumDiagnosticBytes)
                {
                    low = candidate;
                }
                else
                {
                    high = candidate - 1;
                }
            }

            if (low > 0 && char.IsHighSurrogate(message[low - 1]))
            {
                low--;
            }

            return message.Substring(0, low);
        }

        private static void ThrowReportedFailureInEditor(string message)
        {
#if UNITY_EDITOR
            if (Application.isPlaying)
            {
                throw new InvalidOperationException(message);
            }
#endif
        }

        private void LogSlowFrame(
            MasonryRunnerOptions configured,
            TimeSpan frameDuration,
            TimeSpan masonryDuration,
            int payloadBytes
        )
        {
            var fields = new Dictionary<string, string>
            {
                ["duration_ms"] = Milliseconds(frameDuration),
                ["masonry_duration_ms"] = Milliseconds(masonryDuration),
                ["payload_bytes"] = payloadBytes.ToString(CultureInfo.InvariantCulture),
            };
            AddSessionField(fields);
            configured.Logger.Log(
                new MasonryLogRecord(
                    MasonryLogSeverity.Warning,
                    "masonry.frame.slow",
                    "Masonry did work during a slow Unity frame.",
                    fields
                )
            );
        }

        private void AddSessionField(IDictionary<string, string> fields)
        {
            if (lastSession is not null)
            {
                fields["session_id"] = lastSession.Value.Value.ToString();
            }
        }

        private static string Milliseconds(TimeSpan duration) =>
            duration.TotalMilliseconds.ToString("F3", CultureInfo.InvariantCulture);

        private void StopSession(MasonryRunnerOptions configured, bool log)
        {
            preparedAssets?.CancelPending();
            scenes?.BeginSession();
            world?.BeginSession();
            pendingSnapshot = null;
            pendingConnectionEvent = null;
            pendingConnectionMessage = null;
            configured.Transport.Stop();
            state = RunnerState.Stopped;
            inputDisabled = true;
            previousStepTime = null;
            pendingResponses.Clear();
            if (log)
            {
                Log(MasonryLogSeverity.Information, "masonry.host.stopped", "Host stopped.");
            }
        }

        private void LogPendingConnection()
        {
            if (pendingConnectionEvent is null || pendingConnectionMessage is null)
            {
                return;
            }

            Log(MasonryLogSeverity.Information, pendingConnectionEvent, pendingConnectionMessage);
            pendingConnectionEvent = null;
            pendingConnectionMessage = null;
        }

        private static string PlatformName(RuntimePlatform platform) =>
            platform switch
            {
                RuntimePlatform.OSXEditor or RuntimePlatform.OSXPlayer => "macOS",
                RuntimePlatform.WindowsEditor or RuntimePlatform.WindowsPlayer => "Windows",
                RuntimePlatform.LinuxEditor or RuntimePlatform.LinuxPlayer => "Linux",
                RuntimePlatform.IPhonePlayer => "iOS",
                RuntimePlatform.Android => "Android",
                _ => platform.ToString(),
            };

        private MasonryRunnerOptions RequireOptions()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryRunner));
            }

            return options
                ?? throw new InvalidOperationException(
                    "Configure the runner with public host dependencies before use."
                );
        }

        private MasonryRunnerOptions RequireRunningSession()
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (state != RunnerState.Running || lastSession is null)
            {
                throw new InvalidOperationException(
                    "Failures may only be reported while the runner is active."
                );
            }

            return configured;
        }

        private void Log(
            MasonryLogSeverity severity,
            string eventName,
            string message,
            IReadOnlyDictionary<string, string>? fields = null
        ) =>
            RequireOptions().Logger.Log(new MasonryLogRecord(severity, eventName, message, fields));

        private sealed record PendingResponse(
            Response Response,
            bool IsInitial,
            SessionId? PreviousSession
        );

        private sealed class PendingSnapshot
        {
            public PendingSnapshot(Snapshot snapshot, bool isInitial)
            {
                Snapshot = snapshot;
                IsInitial = isInitial;
            }

            public Snapshot Snapshot { get; }

            public bool IsInitial { get; }

            public bool SceneReplacementStarted { get; set; }
        }

        private static bool ReturnMissing(out object? value)
        {
            value = null;
            return false;
        }
    }
}
