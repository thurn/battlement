#nullable enable

using System.Collections.Generic;
using UnityEngine;

namespace Masonry
{
    /// <summary>Severity of one structured Masonry log record.</summary>
    public enum MasonryLogSeverity
    {
        Trace,
        Information,
        Warning,
        Error,
    }

    /// <summary>One structured event emitted by the Masonry host.</summary>
    public sealed record MasonryLogRecord(
        MasonryLogSeverity Severity,
        string EventName,
        string Message,
        IReadOnlyDictionary<string, string>? Fields = null
    );

    /// <summary>Receives structured Masonry log records.</summary>
    public interface IMasonryLogger
    {
        /// <summary>Writes one record.</summary>
        void Log(MasonryLogRecord record);
    }

    /// <summary>Writes structured Masonry records to the Unity console.</summary>
    public sealed class MasonryUnityLogger : IMasonryLogger
    {
        public void Log(MasonryLogRecord record)
        {
            Errors.CheckNotNull(record, nameof(record));

            string message = $"[{record.EventName}] {record.Message}";
            switch (record.Severity)
            {
                case MasonryLogSeverity.Warning:
                    Debug.LogWarning(message);
                    break;
                case MasonryLogSeverity.Error:
                    Debug.LogError(message);
                    break;
                case MasonryLogSeverity.Trace:
                case MasonryLogSeverity.Information:
                default:
                    Debug.Log(message);
                    break;
            }
        }
    }
}
