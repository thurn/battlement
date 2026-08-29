#nullable enable

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    internal abstract record DittoContext;

    internal abstract record BattlementStoredPayload(BattlementLogRecord Record)
    {
        internal sealed record Ordinary(BattlementLogRecord Record)
            : BattlementStoredPayload(Record);

        internal sealed record Context(BattlementLogRecord Record, DittoContext Body)
            : BattlementStoredPayload(Record);
    }

    internal sealed record BattlementLogEntry(
        ulong Sequence,
        DateTimeOffset OccurredAt,
        string Source,
        BattlementStoredPayload Payload
    )
    {
        public BattlementLogRecord Record => Payload.Record;
    }

    internal static class BattlementLogStore
    {
        private const int MaximumRecords = 2_048;
        private static readonly object Gate = new();
        private static readonly List<BattlementLogObserver> Observers = new();
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
            AddPayload(source, new BattlementStoredPayload.Ordinary(Copy(record)), occurredAt);
        }

        public static void AddContext(
            string source,
            BattlementLogRecord record,
            DittoContext body,
            DateTimeOffset? occurredAt = null
        )
        {
            Preconditions.CheckNotNull(record, nameof(record));
            Preconditions.CheckNotNull(body, nameof(body));
            AddPayload(source, new BattlementStoredPayload.Context(Copy(record), body), occurredAt);
        }

        public static BattlementLogObserver Observe()
        {
            lock (Gate)
            {
                var observer = new BattlementLogObserver(Unregister);
                foreach (BattlementLogEntry entry in Records)
                {
                    observer.Accept(entry);
                }

                Observers.Add(observer);
                return observer;
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
                version++;
            }
        }

        private static void AddPayload(
            string source,
            BattlementStoredPayload payload,
            DateTimeOffset? occurredAt
        )
        {
            lock (Gate)
            {
                if (Records.Count == MaximumRecords)
                {
                    Records.Dequeue();
                }

                var entry = new BattlementLogEntry(
                    ++nextSequence,
                    (occurredAt ?? DateTimeOffset.UtcNow).ToUniversalTime(),
                    source,
                    payload
                );
                Records.Enqueue(entry);
                foreach (BattlementLogObserver observer in Observers)
                {
                    observer.Accept(entry);
                }
                version++;
            }
        }

        private static void Unregister(BattlementLogObserver observer)
        {
            lock (Gate)
            {
                Observers.Remove(observer);
            }
        }

        private static BattlementLogRecord Copy(BattlementLogRecord record) =>
            record with
            {
                Fields = record.Fields is null
                    ? null
                    : new ReadOnlyDictionary<string, string>(
                        new Dictionary<string, string>(record.Fields)
                    ),
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
            Write(source, record);
        }

        public static void LogContext(
            string source,
            BattlementLogRecord record,
            DittoContext body,
            DateTimeOffset? occurredAt = null
        )
        {
            BattlementLogStore.AddContext(source, record, body, occurredAt);
            Write(source, record);
        }

        private static void Write(string source, BattlementLogRecord record)
        {
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
