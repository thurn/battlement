#nullable enable

using System;
using UnityEngine;

namespace Battlement.Cloud.Diagnostics
{
    /// <summary>Injectable boundary over Unity's process-global Crash Reporting API.</summary>
    public interface IDiagnosticsBackend
    {
        bool CaptureExceptions { get; set; }
        bool PerformanceReporting { get; set; }
        uint LogBufferSize { set; }
        void SetMetadata(string key, string? value);
    }

    /// <summary>Opt-in Unity Diagnostics capability for one Battlement runner.</summary>
    [CreateAssetMenu(menuName = "Battlement/Diagnostics Module")]
    public sealed class BattlementDiagnosticsModule : BattlementModule
    {
        private const int MaximumLogBufferSize = 50;

        [SerializeField]
        private bool captureExceptions = true;

        [SerializeField, Range(0, MaximumLogBufferSize)]
        private int logBufferSize = 10;

        public override string ModuleId => "battlement.diagnostics";

        public override IBattlementModuleRuntime Prepare() =>
            new BattlementDiagnosticsRuntime(
                new UnityDiagnosticsBackend(),
                captureExceptions,
                checked((uint)logBufferSize)
            );

        private void OnValidate() =>
            logBufferSize = Mathf.Clamp(logBufferSize, 0, MaximumLogBufferSize);
    }

    internal sealed class BattlementDiagnosticsRuntime : IBattlementDiagnosticsRuntime
    {
        private readonly IDiagnosticsBackend backend;
        private bool disposed;
        private string? reportingError;

        public BattlementDiagnosticsRuntime(
            IDiagnosticsBackend backend,
            bool captureExceptions,
            uint logBufferSize
        )
        {
            this.backend = backend ?? throw new ArgumentNullException(nameof(backend));
            if (logBufferSize > 50)
                throw new ArgumentOutOfRangeException(nameof(logBufferSize));
            try
            {
                backend.CaptureExceptions = captureExceptions;
                backend.LogBufferSize = logBufferSize;
            }
            catch (Exception exception)
            {
                throw new InvalidOperationException(
                    "Unity Diagnostics configuration could not be applied.",
                    exception
                );
            }
        }

        public string ModuleId => "battlement.diagnostics";

        public void Execute(DiagnosticsCommand command)
        {
            switch (command)
            {
                case DiagnosticsCommand.SetMetadata metadata:
                    SetMetadata(metadata.Key, metadata.Value);
                    break;
                case DiagnosticsCommand.SetReporting reporting:
                    SetReporting(reporting.Enabled);
                    break;
                default:
                    throw new BattlementModuleException(
                        CoreErrorCode.InvalidEncoding,
                        "The Diagnostics command kind is unknown."
                    );
            }
        }

        public void SetMetadata(string key, string? value)
        {
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementDiagnosticsRuntime));
            CoreErrorCode? validation = DiagnosticsProtocol.Validate(key, value);
            if (validation is CoreErrorCode errorCode)
            {
                throw new BattlementModuleException(
                    errorCode,
                    "The Diagnostics metadata command is invalid."
                );
            }
            try
            {
                backend.SetMetadata(key, value);
            }
            catch (Exception exception)
            {
                throw new BattlementModuleException(
                    CoreErrorCode.DiagnosticsOperationFailed,
                    "Unity Diagnostics metadata could not be updated.",
                    exception
                );
            }
        }

        public void SetReporting(bool enabled)
        {
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementDiagnosticsRuntime));
            reportingError = null;
            try
            {
                backend.CaptureExceptions = enabled;
                backend.PerformanceReporting = enabled;
            }
            catch (Exception exception)
            {
                reportingError = "A local reporting control could not be applied.";
                throw new BattlementModuleException(
                    CoreErrorCode.DiagnosticsOperationFailed,
                    reportingError,
                    exception
                );
            }
        }

        public DiagnosticsObservation ReadReporting()
        {
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementDiagnosticsRuntime));
            try
            {
                return new DiagnosticsObservation(
                    backend.CaptureExceptions,
                    backend.PerformanceReporting,
                    reportingError
                );
            }
            catch (Exception)
            {
                return new DiagnosticsObservation(
                    null,
                    null,
                    "Local reporting controls could not be observed."
                );
            }
        }

        public void Dispose() => disposed = true;
    }
}
