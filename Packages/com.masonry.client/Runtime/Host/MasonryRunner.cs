#nullable enable

using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
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
        private readonly Queue<PendingResponse> pendingResponses = new();
        private TimeSpan? previousStepTime;
        private RunnerState state;
        private SessionId? lastSession;
        private bool inputDisabled = true;
        private bool isProcessingResponses;
        private bool wasPaused;
        private bool isDisposed;

        private const int MaximumResponseBytes = 16 * 1024 * 1024;
        private const int MaximumQueuedResponses = 256;
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
                options?.AssetStorage.Dispose();
            }
            finally
            {
                options?.Transport.Dispose();
                isDisposed = true;
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
            inputDisabled = true;
            previousStepTime = configured.Clock.Elapsed;

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
                    Log(
                        MasonryLogSeverity.Information,
                        reconnecting ? "masonry.host.reconnected" : "masonry.host.connected",
                        reconnecting ? "Host reconnected." : "Host connected."
                    );
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

            lastSession = responseSession;
            state = RunnerState.ApplyingSnapshot;
            inputDisabled = snapshot.IsInputDisabled;
            state = RunnerState.Running;
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
    }
}
