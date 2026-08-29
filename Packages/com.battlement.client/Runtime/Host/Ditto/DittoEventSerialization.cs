#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal static class DittoEventSerialization
    {
        private static readonly HashSet<string> ProtectedStrings = new()
        {
            "artifact_id",
            "battlement_error_id",
            "capture_adapter",
            "code",
            "context",
            "engine_session_id",
            "error_id",
            "error_ref",
            "event_name",
            "execution_status",
            "expired_deadline",
            "job_id",
            "kind",
            "player_session_id",
            "platform",
            "run_id",
            "scenario_id",
            "severity",
            "sha256",
            "source",
            "stage",
            "state",
            "status",
        };

        public static byte[] Encode(
            BattlementLogEntry entry,
            string jobId,
            string playerSessionId,
            IReadOnlyList<string> redactions
        )
        {
            DittoEventRecord record = entry.Payload switch
            {
                BattlementStoredPayload.Context context => new DittoContextRecord(
                    1,
                    jobId,
                    playerSessionId,
                    entry.Sequence,
                    TimestampUnixUs(entry.OccurredAt),
                    Source(entry.Source),
                    Severity(entry.Record.Severity),
                    entry.Record.EventName,
                    entry.Record.Message,
                    context.Body
                ),
                BattlementStoredPayload.Ordinary => new DittoOrdinaryLogRecord(
                    1,
                    jobId,
                    playerSessionId,
                    entry.Sequence,
                    TimestampUnixUs(entry.OccurredAt),
                    Source(entry.Source),
                    Severity(entry.Record.Severity),
                    entry.Record.EventName,
                    entry.Record.Message,
                    entry.Record.Fields ?? new Dictionary<string, string>(),
                    entry.Record.Exception?.ToString(),
                    entry.Record.StackTrace
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(entry)),
            };
            JObject value = JObject.Parse(
                Encoding.UTF8.GetString(DittoLifecycleCodec.Encode<DittoEventRecord>(record))
            );
            Redact(value, redactions, false);
            return Encoding.UTF8.GetBytes(value.ToString(Formatting.None) + "\n");
        }

        private static void Redact(
            JToken token,
            IReadOnlyList<string> redactions,
            bool freeFormValues
        )
        {
            foreach (JProperty property in token.Children<JProperty>())
            {
                bool protectedValue = Protected(property.Name) || ProtectedReason(property);
                if (property.Value.Type == JTokenType.String && (freeFormValues || !protectedValue))
                {
                    string value = property.Value.Value<string>() ?? string.Empty;
                    foreach (string secret in redactions)
                    {
                        value = value.Replace(secret, "<redacted>");
                    }
                    property.Value = value;
                    continue;
                }
                Redact(property.Value, redactions, property.Name == "fields");
            }
            foreach (JToken child in token.Children().Where(child => child is not JProperty))
            {
                Redact(child, redactions, freeFormValues);
            }
        }

        private static bool ProtectedReason(JProperty property) =>
            property.Name == "reason"
            && property.Parent?["context"]?.Value<string>() == "job-ended";

        private static bool Protected(string name) =>
            ProtectedStrings.Contains(name)
            || name.EndsWith("_id", StringComparison.Ordinal)
            || name.EndsWith("_ids", StringComparison.Ordinal)
            || name.EndsWith("_ref", StringComparison.Ordinal)
            || name.EndsWith("_refs", StringComparison.Ordinal)
            || name.EndsWith("_sha256", StringComparison.Ordinal);

        private static long TimestampUnixUs(DateTimeOffset value) =>
            checked((value.UtcDateTime.Ticks - DateTime.UnixEpoch.Ticks) / 10);

        private static DittoLogSource Source(string value) =>
            value switch
            {
                "battlement" => DittoLogSource.Battlement,
                "rust" => DittoLogSource.Rust,
                "unity" => DittoLogSource.Unity,
                "ditto-player" => DittoLogSource.DittoPlayer,
                _ => throw new ArgumentOutOfRangeException(nameof(value), value, "Unknown source."),
            };

        private static DittoLogSeverity Severity(BattlementLogSeverity value) =>
            value switch
            {
                BattlementLogSeverity.Trace => DittoLogSeverity.Trace,
                BattlementLogSeverity.Debug => DittoLogSeverity.Debug,
                BattlementLogSeverity.Information => DittoLogSeverity.Information,
                BattlementLogSeverity.Warning => DittoLogSeverity.Warning,
                BattlementLogSeverity.Error => DittoLogSeverity.Error,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };
    }
}
