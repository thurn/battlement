#nullable enable

using System;
using System.Collections.Generic;
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
        private bool isConnected;
        private bool isDisposed;

        public MasonryTransportKind TransportKind => transportKind;

        public MasonryNativeTransportConfiguration NativeTransport => nativeTransport;

        public MasonryHttpTransportConfiguration HttpTransport => httpTransport;

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
            if (isConnected)
            {
                throw new InvalidOperationException("The runner is already connected.");
            }

            configured.Transport.Connect();
            isConnected = true;
            previousStepTime = configured.Clock.Elapsed;
            Log(MasonryLogSeverity.Information, "masonry.host.connected", "Host connected.");
        }

        /// <summary>Stops the current session and starts a new one on the same transport.</summary>
        public void Reconnect()
        {
            MasonryRunnerOptions configured = RequireOptions();
            if (isConnected)
            {
                configured.Transport.Stop();
            }

            configured.Transport.Connect();
            isConnected = true;
            previousStepTime = configured.Clock.Elapsed;
            Log(MasonryLogSeverity.Information, "masonry.host.reconnected", "Host reconnected.");
        }

        /// <summary>Stops the active session. Repeated calls are no-ops.</summary>
        public void Stop()
        {
            if (!isConnected || options is null)
            {
                return;
            }

            options.Transport.Stop();
            isConnected = false;
            previousStepTime = null;
            Log(MasonryLogSeverity.Information, "masonry.host.stopped", "Host stopped.");
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
            if (!isConnected || options is null)
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

        private void OnDestroy() => Dispose();

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
