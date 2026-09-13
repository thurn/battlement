#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement
{
    internal static class BattlementNativeLogging
    {
        private const int Ok = 0;
        private const ulong MaximumPayloadBytes = 5 * 1024 * 1024;
        private static readonly object Gate = new();
        private static readonly UTF8Encoding Utf8 = new(false, true);
        private static bool active = true;

        public static void Drain()
        {
            lock (Gate)
            {
                if (!active)
                {
                    return;
                }

                ulong output = 0;
                try
                {
                    int status = BattlementNativeMethods.battlement_logging_drain(out output);
                    string records = Decode(Inspect(output));
                    if (status != Ok)
                    {
                        throw new IOException($"Native logging drain failed: {records}");
                    }

                    foreach (string line in records.Split('\n'))
                    {
                        if (!string.IsNullOrWhiteSpace(line))
                        {
                            Emit(JObject.Parse(line));
                        }
                    }
                }
                catch (Exception exception)
                {
                    Disable(exception);
                }
                finally
                {
                    Release(output);
                }
            }
        }

        private static unsafe string Decode(BattlementNativeBuffer buffer)
        {
            string? shapeError = buffer.ValidateShape(MaximumPayloadBytes);
            if (shapeError is not null)
            {
                throw new IOException(shapeError);
            }
            if (buffer.Length == 0)
            {
                return string.Empty;
            }

            return Utf8.GetString(
                new ReadOnlySpan<byte>(buffer.Data.ToPointer(), checked((int)buffer.Length))
            );
        }

        private static void Disable(Exception exception)
        {
            active = false;
            BattlementUnityLogging.Log(
                "battlement",
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    "battlement.logging.failed",
                    "Rust tracing could not be forwarded to Unity.",
                    Exception: exception
                )
            );
        }

        private static void Emit(JObject value)
        {
            BattlementUnityLogging.Log(
                "rust",
                new BattlementLogRecord(
                    Severity(RequiredString(value, "severity")),
                    RequiredString(value, "event_name"),
                    RequiredString(value, "message"),
                    Fields(value["fields"] as JObject)
                ),
                OccurredAt(value.Value<ulong?>("timestamp_unix_us"))
            );
        }

        private static IReadOnlyDictionary<string, string> Fields(JObject? fields) =>
            fields
                ?.Properties()
                .ToDictionary(
                    field => field.Name,
                    field =>
                        field.Value.Type == JTokenType.String
                            ? field.Value.Value<string>() ?? string.Empty
                            : field.Value.ToString(Formatting.None)
                )
            ?? new Dictionary<string, string>();

        private static BattlementNativeBuffer Inspect(ulong handle)
        {
            if (handle == 0)
            {
                return default;
            }
            int status = BattlementNativeMethods.battlement_buffer_info(
                handle,
                out IntPtr data,
                out ulong length,
                out ulong allocationBytes
            );
            if (status != Ok)
            {
                throw new IOException($"Native logging buffer inspection failed: {status}.");
            }
            return new BattlementNativeBuffer(handle, data, length, allocationBytes);
        }

        private static void Release(ulong handle)
        {
            if (handle != 0 && BattlementNativeMethods.battlement_release_buffer(handle) != Ok)
            {
                throw new IOException("Native logging buffer release failed.");
            }
        }

        private static DateTimeOffset OccurredAt(ulong? timestampUnixUs)
        {
            if (timestampUnixUs is null || timestampUnixUs > long.MaxValue)
            {
                return DateTimeOffset.UtcNow;
            }

            return DateTimeOffset.FromUnixTimeMilliseconds((long)timestampUnixUs.Value / 1_000);
        }

        private static string RequiredString(JObject value, string name) =>
            value.Value<string>(name)
            ?? throw new InvalidDataException($"Native logging record has no {name}.");

        private static BattlementLogSeverity Severity(string value) =>
            value switch
            {
                "trace" => BattlementLogSeverity.Trace,
                "debug" => BattlementLogSeverity.Debug,
                "information" => BattlementLogSeverity.Information,
                "warning" => BattlementLogSeverity.Warning,
                "error" => BattlementLogSeverity.Error,
                _ => throw new InvalidDataException(
                    $"Native logging record has unknown severity {value}."
                ),
            };
    }
}
