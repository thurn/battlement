#nullable enable

using UnityEngine.CrashReportHandler;

namespace Battlement.Cloud.Diagnostics
{
    internal sealed class UnityDiagnosticsBackend : IDiagnosticsBackend
    {
        public bool CaptureExceptions
        {
            get
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                return CrashReportHandler.enableCaptureExceptions;
#else
                throw new System.PlatformNotSupportedException();
#endif
            }
            set
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                CrashReportHandler.enableCaptureExceptions = value;
#endif
            }
        }

        public bool PerformanceReporting
        {
            get
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                return UnityEngine.Analytics.PerformanceReporting.enabled;
#else
                throw new System.PlatformNotSupportedException();
#endif
            }
            set
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                UnityEngine.Analytics.PerformanceReporting.enabled = value;
#else
                throw new System.PlatformNotSupportedException();
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
