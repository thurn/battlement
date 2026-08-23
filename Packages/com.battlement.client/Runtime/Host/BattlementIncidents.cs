#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Newtonsoft.Json;
using UnityEngine;

namespace Battlement
{
    /// <summary>How much runtime state remains safe after an incident.</summary>
    public enum BattlementFailureDisposition
    {
        Logged,
        CommandFailed,
        SessionFailed,
        RestartRequired,
    }

    /// <summary>Subsystem in which an incident originated.</summary>
    public enum BattlementIncidentSource
    {
        Unity,
        Native,
        Transport,
        Protocol,
        Host,
    }

    /// <summary>Product-neutral failure category suitable for game-owned presentation.</summary>
    public enum BattlementPlayerFailureKind
    {
        ContinueAllowed,
        RestartRequired,
    }

    /// <summary>Minimal, nontechnical failure information that may be shown to a player.</summary>
    public sealed record BattlementPlayerFailure(
        BattlementPlayerFailureKind Kind,
        string IncidentId
    );

    /// <summary>One correlated diagnostic captured by the Battlement runtime.</summary>
    public sealed record BattlementIncident(
        string Id,
        DateTimeOffset OccurredAt,
        BattlementFailureDisposition Disposition,
        BattlementIncidentSource Source,
        string EventName,
        string Message,
        Exception? Exception,
        string? StackTrace,
        IReadOnlyDictionary<string, string> Fields,
        IReadOnlyList<BattlementLogRecord> RecentRecords
    );

    /// <summary>Receives complete developer diagnostics for one incident.</summary>
    public interface IBattlementIncidentSink
    {
        /// <summary>Persists or forwards an incident.</summary>
        void Report(BattlementIncident incident);
    }

    /// <summary>Displays product-owned UI for a runtime failure.</summary>
    public interface IBattlementFailurePresenter
    {
        /// <summary>Shows one nontechnical failure.</summary>
        void Show(BattlementPlayerFailure failure);

        /// <summary>Clears the current failure presentation.</summary>
        void Hide();
    }

    /// <summary>
    /// Writes bounded incident reports beneath the application's persistent data path.
    /// </summary>
    public sealed class BattlementFileIncidentSink : IBattlementIncidentSink
    {
        private const int MaximumReports = 20;
        private readonly string directory;

        public BattlementFileIncidentSink()
            : this(Path.Combine(Application.persistentDataPath, "Battlement", "Incidents")) { }

        /// <summary>Creates a bounded file sink rooted at a custom directory.</summary>
        public BattlementFileIncidentSink(string directory)
        {
            if (string.IsNullOrWhiteSpace(directory))
            {
                throw new ArgumentException(
                    "An incident directory is required.",
                    nameof(directory)
                );
            }

            this.directory = directory;
        }

        public void Report(BattlementIncident incident)
        {
            Errors.CheckNotNull(incident, nameof(incident));
            Directory.CreateDirectory(directory);
            string path = Path.Combine(directory, $"{incident.Id}.json");
            File.WriteAllText(
                path,
                JsonConvert.SerializeObject(ToReport(incident), Formatting.Indented)
            );
            foreach (
                FileInfo stale in new DirectoryInfo(directory)
                    .GetFiles("*.json")
                    .OrderByDescending(file => file.CreationTimeUtc)
                    .Skip(MaximumReports)
            )
            {
                stale.Delete();
            }
        }

        private static object ToReport(BattlementIncident incident) =>
            new
            {
                incident.Id,
                incident.OccurredAt,
                disposition = incident.Disposition.ToString(),
                source = incident.Source.ToString(),
                event_name = incident.EventName,
                incident.Message,
                exception = incident.Exception?.ToString(),
                stack_trace = incident.StackTrace,
                incident.Fields,
                recent_records = incident.RecentRecords.Select(record => new
                {
                    severity = record.Severity.ToString(),
                    event_name = record.EventName,
                    record.Message,
                    record.Fields,
                    exception = record.Exception?.ToString(),
                }),
            };
    }

    internal sealed class BattlementIncidentReporter : IBattlementLogger
    {
        private const int MaximumRecentRecords = 128;
        private readonly Queue<BattlementLogRecord> recent = new();
        private readonly IBattlementLogger logger;
        private readonly IBattlementIncidentSink sink;
        private readonly Action<BattlementIncident>? showDevelopmentIncident;

        public BattlementIncidentReporter(
            IBattlementLogger logger,
            IBattlementIncidentSink sink,
            Action<BattlementIncident>? showDevelopmentIncident = null
        ) =>
            (this.logger, this.sink, this.showDevelopmentIncident) = (
                logger,
                sink,
                showDevelopmentIncident
            );

        public void Log(BattlementLogRecord record)
        {
            if (recent.Count == MaximumRecentRecords)
            {
                recent.Dequeue();
            }

            recent.Enqueue(record);
            logger.Log(record);
        }

        public BattlementIncident Report(
            BattlementFailureDisposition disposition,
            BattlementIncidentSource source,
            string eventName,
            string message,
            Exception? exception = null,
            string? stackTrace = null,
            IReadOnlyDictionary<string, string>? fields = null
        )
        {
            string id = IncidentId();
            var diagnosticFields = new Dictionary<string, string>(
                fields ?? new Dictionary<string, string>()
            )
            {
                ["disposition"] = disposition.ToString(),
                ["incident_id"] = id,
                ["source"] = source.ToString(),
            };
            if (exception is not null)
            {
                diagnosticFields["exception_type"] = exception.GetType().FullName ?? string.Empty;
            }

            var incident = new BattlementIncident(
                id,
                DateTimeOffset.UtcNow,
                disposition,
                source,
                eventName,
                message,
                exception,
                stackTrace ?? exception?.StackTrace,
                diagnosticFields,
                recent.ToArray()
            );
            Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    eventName,
                    message,
                    diagnosticFields,
                    exception,
                    incident.StackTrace
                )
            );
            try
            {
                sink.Report(incident);
            }
            catch (Exception sinkException)
            {
                Log(
                    new BattlementLogRecord(
                        BattlementLogSeverity.Warning,
                        "battlement.incident.sink_failed",
                        "The incident sink could not record a diagnostic.",
                        new Dictionary<string, string> { ["incident_id"] = id },
                        sinkException
                    )
                );
            }
            ShowDevelopmentIncident(incident);
            return incident;
        }

        private static string IncidentId()
        {
            string value = Convert.ToBase64String(Guid.NewGuid().ToByteArray());
            return value.TrimEnd('=').Replace('+', '-').Replace('/', '_');
        }

        private void ShowDevelopmentIncident(BattlementIncident incident)
        {
            if (
                showDevelopmentIncident is null
                || incident.Source
                    is not BattlementIncidentSource.Unity
                        and not BattlementIncidentSource.Native
            )
            {
                return;
            }

            try
            {
                showDevelopmentIncident(incident);
            }
            catch (Exception exception)
            {
                Log(
                    new BattlementLogRecord(
                        BattlementLogSeverity.Warning,
                        "battlement.development_dialog.failed",
                        "The development error dialog could not show an incident.",
                        new Dictionary<string, string> { ["incident_id"] = incident.Id },
                        exception
                    )
                );
            }
        }
    }

    internal sealed record BattlementCapturedUnityFault(
        string Condition,
        string StackTrace,
        LogType Type,
        Exception? Exception = null,
        bool IsExplicit = false
    );

    internal static class BattlementUnityFaults
    {
        private static readonly object Gate = new();
        private static readonly List<Action<BattlementCapturedUnityFault>> Subscribers = new();
        private static bool installed;

        public static IDisposable Subscribe(Action<BattlementCapturedUnityFault> subscriber)
        {
            lock (Gate)
            {
                EnsureInstalled();
                Subscribers.Add(subscriber);
            }

            return new Subscription(subscriber);
        }

        private static void EnsureInstalled()
        {
            if (installed)
            {
                return;
            }

            Application.logMessageReceivedThreaded += Receive;
            AppDomain.CurrentDomain.UnhandledException += (_, arguments) =>
            {
                string condition = arguments.ExceptionObject?.ToString() ?? "Unhandled exception";
                string stackTrace =
                    (arguments.ExceptionObject as Exception)?.StackTrace ?? string.Empty;
                Receive(condition, stackTrace, LogType.Exception);
            };
            TaskScheduler.UnobservedTaskException += (_, arguments) =>
                Receive(
                    arguments.Exception.ToString(),
                    arguments.Exception.StackTrace ?? string.Empty,
                    LogType.Exception
                );
            installed = true;
        }

        private static void Receive(string condition, string stackTrace, LogType type)
        {
            if (type is not LogType.Exception and not LogType.Assert)
            {
                return;
            }

            Action<BattlementCapturedUnityFault>[] subscribers;
            lock (Gate)
            {
                subscribers = Subscribers.ToArray();
            }

            var fault = new BattlementCapturedUnityFault(condition, stackTrace, type);
            foreach (Action<BattlementCapturedUnityFault> subscriber in subscribers)
            {
                subscriber(fault);
            }
        }

        private sealed class Subscription : IDisposable
        {
            private Action<BattlementCapturedUnityFault>? subscriber;

            public Subscription(Action<BattlementCapturedUnityFault> subscriber) =>
                this.subscriber = subscriber;

            public void Dispose()
            {
                Action<BattlementCapturedUnityFault>? current = subscriber;
                if (current is null)
                {
                    return;
                }

                lock (Gate)
                {
                    Subscribers.Remove(current);
                }

                subscriber = null;
            }
        }
    }
}
