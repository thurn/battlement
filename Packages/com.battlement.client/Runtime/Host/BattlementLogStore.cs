#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    internal sealed record BattlementLogEntry(
        ulong Sequence,
        DateTimeOffset OccurredAt,
        string Source,
        BattlementLogRecord Record
    );

    internal static class BattlementLogStore
    {
        private const int MaximumRecords = 2_048;
        private static readonly object Gate = new();
        private static readonly Queue<BattlementLogEntry> Records = new();
        private static ulong nextSequence;
        private static ulong version;

        public static void Add(
            string source,
            BattlementLogRecord record,
            DateTimeOffset? occurredAt = null
        )
        {
            Preconditions.CheckNotNull(record, nameof(record));
            lock (Gate)
            {
                if (Records.Count == MaximumRecords)
                {
                    Records.Dequeue();
                }

                Records.Enqueue(
                    new BattlementLogEntry(
                        ++nextSequence,
                        occurredAt ?? DateTimeOffset.UtcNow,
                        source,
                        Copy(record)
                    )
                );
                version++;
            }
        }

        public static BattlementLogEntry[] Snapshot(out ulong currentVersion)
        {
            lock (Gate)
            {
                currentVersion = version;
                return Records.ToArray();
            }
        }

        public static BattlementLogRecord[] RecentRecords(int maximum)
        {
            lock (Gate)
            {
                return Records
                    .Skip(Math.Max(0, Records.Count - maximum))
                    .Select(entry => entry.Record)
                    .ToArray();
            }
        }

        internal static void Clear()
        {
            lock (Gate)
            {
                Records.Clear();
                nextSequence = 0;
                version++;
            }
        }

        private static BattlementLogRecord Copy(BattlementLogRecord record) =>
            record with
            {
                Fields = record.Fields is null
                    ? null
                    : new Dictionary<string, string>(record.Fields),
            };
    }

    internal static class BattlementUnityLogging
    {
        private const string ForwardedPrefix = "[Battlement/";

        public static bool IsForwarded(string condition) =>
            condition.StartsWith(ForwardedPrefix, StringComparison.Ordinal);

        public static void Log(
            string source,
            BattlementLogRecord record,
            DateTimeOffset? occurredAt = null
        )
        {
            BattlementLogStore.Add(source, record, occurredAt);
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
            string message =
                $"{ForwardedPrefix}{SourceName(source)}][{record.EventName}] "
                + record.Message
                + fields;
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
                case BattlementLogSeverity.Debug:
                case BattlementLogSeverity.Information:
                default:
                    Debug.Log(message);
                    break;
            }
        }

        private static string SourceName(string source) =>
            source switch
            {
                "battlement" => "Managed",
                "" => "Unknown",
                _ => char.ToUpperInvariant(source[0]) + source.Substring(1),
            };
    }
}
