#nullable enable

using System;
using System.Collections.Generic;
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
        private TimeSpan? previousStepTime;
        private RunnerState state;
        private SessionId? lastSession;
        private bool inputDisabled = true;
        private bool wasPaused;
        private bool isDisposed;

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
            TimeSpan now = configured.Clock.Elapsed;
            TimeSpan previous = previousStepTime ?? now;
            if (now < previous)
            {
                throw new InvalidOperationException("The Masonry clock must be monotonic.");
            }

            previousStepTime = now;
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

            try
            {
                Connect connect = BuildConnect(configured);
                byte[] bytes = configured.ProtocolCodec.SerializeConnect(connect);
                MasonryTransportResult result = configured.Transport.Connect(bytes);
                if (result.Status != MasonryTransportStatus.Success)
                {
                    FailSession(configured, "Connect failed.", result);
                    return;
                }

                Response response = configured.ProtocolCodec.DeserializeResponse(result.Payload);
                AcceptInitialResponse(configured, response, previousSession);
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
                FailSession(configured, $"Connect response failed: {exception.Message}");
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

        private void AcceptInitialResponse(
            MasonryRunnerOptions configured,
            Response response,
            SessionId? previousSession
        )
        {
            if (previousSession is not null && response.SessionId == previousSession.Value)
            {
                Log(
                    MasonryLogSeverity.Warning,
                    "masonry.response.wrong_session",
                    "Discarded a response from the previous session."
                );
                return;
            }

            if (
                response.Messages.Count == 0
                || response.Messages[0]
                    is not ResponseMessage<Command>.SnapshotMessage snapshotMessage
            )
            {
                FailSession(configured, "The first current-session message was not a snapshot.");
                return;
            }

            if (snapshotMessage.Snapshot.SessionId != response.SessionId)
            {
                FailSession(configured, "The initial snapshot used the wrong session.");
                return;
            }

            lastSession = response.SessionId;
            state = RunnerState.ApplyingSnapshot;
            inputDisabled = snapshotMessage.Snapshot.IsInputDisabled;
            state = RunnerState.Running;
        }

        private void FailSession(
            MasonryRunnerOptions configured,
            string message,
            MasonryTransportResult? result = null
        )
        {
            var fields = new Dictionary<string, string>();
            if (result is not null)
            {
                fields["status"] = result.Status.ToString();
                if (!string.IsNullOrEmpty(result.Diagnostic))
                {
                    fields["diagnostic"] = result.Diagnostic!;
                }
            }

            Log(MasonryLogSeverity.Error, "masonry.session.failed", message, fields);
            StopSession(configured, false);
        }

        private void StopSession(MasonryRunnerOptions configured, bool log)
        {
            configured.Transport.Stop();
            state = RunnerState.Stopped;
            inputDisabled = true;
            previousStepTime = null;
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
    }
}
