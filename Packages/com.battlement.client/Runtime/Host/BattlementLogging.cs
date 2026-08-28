#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>Severity of one structured Battlement log record.</summary>
    public enum BattlementLogSeverity
    {
        Trace,
        Debug,
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

    internal interface IBattlementLogHistory
    {
        IReadOnlyList<BattlementLogRecord> RecentRecords(int maximum);
    }

    /// <summary>Writes structured Battlement records to the Unity console.</summary>
    public sealed class BattlementUnityLogger : IBattlementLogger, IBattlementLogHistory
    {
        public void Log(BattlementLogRecord record)
        {
            Preconditions.CheckNotNull(record, nameof(record));
            BattlementUnityLogging.Log("battlement", record);
        }

        IReadOnlyList<BattlementLogRecord> IBattlementLogHistory.RecentRecords(int maximum) =>
            BattlementLogStore.RecentRecords(maximum);
    }
}
