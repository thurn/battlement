#nullable enable

using UnityEngine.CrashReportHandler;

namespace Battlement.Cloud.Diagnostics
{
    internal sealed class UnityDiagnosticsBackend : IDiagnosticsBackend
    {
        public bool CaptureExceptions
        {
            set
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                CrashReportHandler.enableCaptureExceptions = value;
#endif
            }
        }

        public uint LogBufferSize
        {
            set
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                CrashReportHandler.logBufferSize = value;
#endif
            }
        }

        public void SetMetadata(string key, string? value)
        {
#if !UNITY_WEBGL || UNITY_EDITOR
            CrashReportHandler.SetUserMetadata(key, value);
#endif
        }
    }
}
