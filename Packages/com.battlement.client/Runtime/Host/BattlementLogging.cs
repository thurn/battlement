#nullable enable

using System.Collections.Generic;
using UnityEngine;

namespace Battlement
{
    /// <summary>Severity of one structured Battlement log record.</summary>
    public enum BattlementLogSeverity
    {
        Trace,
        Information,
        Warning,
        Error,
    }

    /// <summary>One structured event emitted by the Battlement host.</summary>
    public sealed record BattlementLogRecord(
        BattlementLogSeverity Severity,
        string EventName,
        string Message,
        IReadOnlyDictionary<string, string>? Fields = null
    );

    /// <summary>Receives structured Battlement log records.</summary>
    public interface IBattlementLogger
    {
        /// <summary>Writes one record.</summary>
        void Log(BattlementLogRecord record);
    }

    /// <summary>Writes structured Battlement records to the Unity console.</summary>
    public sealed class BattlementUnityLogger : IBattlementLogger
    {
        public void Log(BattlementLogRecord record)
        {
            Errors.CheckNotNull(record, nameof(record));

            string message = $"[{record.EventName}] {record.Message}";
            switch (record.Severity)
            {
                case BattlementLogSeverity.Warning:
                    Debug.LogWarning(message);
                    break;
                case BattlementLogSeverity.Error:
                    Debug.LogError(message);
                    break;
                case BattlementLogSeverity.Trace:
                case BattlementLogSeverity.Information:
                default:
                    Debug.Log(message);
                    break;
            }
        }
    }
}
