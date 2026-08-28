#nullable enable

using System;
using UnityEngine;

namespace Battlement.Cloud.Diagnostics
{
    /// <summary>Injectable boundary over Unity's process-global Crash Reporting API.</summary>
    public interface IDiagnosticsBackend
    {
        bool CaptureExceptions { set; }
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
            if (disposed)
                throw new ObjectDisposedException(nameof(BattlementDiagnosticsRuntime));
            CoreErrorCode? validation = DiagnosticsProtocol.Validate(command);
            if (validation is CoreErrorCode errorCode)
            {
                throw new BattlementModuleException(
                    errorCode,
                    "The Diagnostics metadata command is invalid."
                );
            }
            if (command is not DiagnosticsCommand.SetMetadata metadata)
            {
                throw new BattlementModuleException(
                    CoreErrorCode.InvalidEncoding,
                    "The Diagnostics command kind is unknown."
                );
            }
            try
            {
                backend.SetMetadata(metadata.Key, metadata.Value);
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

        public void Dispose() => disposed = true;
    }
}
