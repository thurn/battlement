#nullable enable

using UnityEngine.CrashReportHandler;

namespace Battlement.Cloud.Diagnostics
{
    internal sealed class UnityDiagnosticsBackend : IDiagnosticsBackend
    {
        public bool CaptureExceptions
        {
            set => CrashReportHandler.enableCaptureExceptions = value;
        }

        public uint LogBufferSize
        {
            set => CrashReportHandler.logBufferSize = value;
        }

        public void SetMetadata(string key, string? value) =>
            CrashReportHandler.SetUserMetadata(key, value);
    }
}
