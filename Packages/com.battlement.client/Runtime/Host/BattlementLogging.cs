#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
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
        IReadOnlyDictionary<string, string>? Fields = null,
        Exception? Exception = null,
        string? StackTrace = null
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
            ArgumentChecks.CheckNotNull(record, nameof(record));

            string fields =
                record.Fields is null || record.Fields.Count == 0
                    ? string.Empty
                    : "\n"
                        + string.Join(
                            " ",
                            record
                                .Fields.OrderBy(field => field.Key)
                                .Select(field => $"{field.Key}={field.Value}")
                        );
            string diagnostic = record.Exception?.ToString() ?? record.StackTrace ?? string.Empty;
            string message = $"[{record.EventName}] {record.Message}{fields}";
            if (!string.IsNullOrWhiteSpace(diagnostic))
            {
                message += $"\n{diagnostic}";
            }
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
