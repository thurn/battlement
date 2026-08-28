#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Newtonsoft.Json;
using UnityEngine;

namespace Battlement.Errors
{
    /// <summary>How much runtime state remains safe after an error.</summary>
    public enum BattlementErrorType
    {
        Logged,
        CommandFailed,
        SessionFailed,
        RestartRequired,
    }

    /// <summary>Subsystem in which an error originated.</summary>
    public enum BattlementErrorSource
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
    public sealed record BattlementPlayerFailure(BattlementPlayerFailureKind Kind, string ErrorId);

    /// <summary>One correlated diagnostic captured by the Battlement runtime.</summary>
    public sealed record BattlementError(
        string Id,
        DateTimeOffset OccurredAt,
        BattlementErrorType Type,
        BattlementErrorSource Source,
        string EventName,
        string Message,
        Exception? Exception,
        string? StackTrace,
        IReadOnlyDictionary<string, string> Fields,
        IReadOnlyList<BattlementLogRecord> RecentRecords
    )
    {
        internal string? AnsiStackTrace { get; init; }
    }

    /// <summary>Receives complete developer diagnostics for one error.</summary>
    public interface IBattlementErrorSink
    {
        /// <summary>Persists or forwards an error.</summary>
        void Report(BattlementError error);
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
    /// Writes bounded error reports beneath the application's persistent data path.
    /// </summary>
    public sealed class BattlementFileErrorSink : IBattlementErrorSink
    {
        private const int MaximumReports = 20;
        private readonly string directory;

        public BattlementFileErrorSink()
            : this(Path.Combine(Application.persistentDataPath, "Battlement", "Errors")) { }

        /// <summary>Creates a bounded file sink rooted at a custom directory.</summary>
        public BattlementFileErrorSink(string directory)
        {
            if (string.IsNullOrWhiteSpace(directory))
            {
                throw new ArgumentException("An error directory is required.", nameof(directory));
            }

            this.directory = directory;
        }

        public void Report(BattlementError error)
        {
            Preconditions.CheckNotNull(error, nameof(error));
            Directory.CreateDirectory(directory);
            string path = Path.Combine(directory, $"{error.Id}.json");
            File.WriteAllText(
                path,
                JsonConvert.SerializeObject(ToReport(error), Formatting.Indented)
            );
            foreach (
                FileInfo stale in new DirectoryInfo(directory)
                    .GetFiles("*.json")
                    .OrderByDescending(file => file.LastWriteTimeUtc)
                    .ThenByDescending(file => file.Name, StringComparer.Ordinal)
                    .Skip(MaximumReports)
            )
            {
                stale.Delete();
            }
        }

        private static object ToReport(BattlementError error) =>
            new
            {
                error.Id,
                error.OccurredAt,
                type = error.Type.ToString(),
                source = error.Source.ToString(),
                event_name = error.EventName,
                error.Message,
                exception = error.Exception?.ToString(),
                stack_trace = error.StackTrace,
                error.Fields,
                recent_records = error.RecentRecords.Select(record => new
                {
                    severity = record.Severity.ToString(),
                    event_name = record.EventName,
                    record.Message,
                    record.Fields,
                    exception = record.Exception?.ToString(),
                }),
            };
    }

    internal sealed class BattlementErrorReporter : IBattlementLogger
    {
        private const int MaximumRecentRecords = 128;
        private readonly Queue<BattlementLogRecord> recent = new();
        private readonly IBattlementLogger logger;
        private readonly IBattlementErrorSink sink;
        private readonly Action<BattlementError>? showDevelopmentError;
        private readonly IBattlementCaughtFailureReporter caughtFailureReporter;
        private readonly HashSet<string> reportedCaughtFailures = new(StringComparer.Ordinal);

        public BattlementErrorReporter(
            IBattlementLogger logger,
            IBattlementErrorSink sink,
            Action<BattlementError>? showDevelopmentError = null,
            IBattlementCaughtFailureReporter? caughtFailureReporter = null
        ) =>
            (this.logger, this.sink, this.showDevelopmentError, this.caughtFailureReporter) = (
                logger,
                sink,
                showDevelopmentError,
                caughtFailureReporter ?? new UnityCaughtFailureReporter()
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

        public BattlementError Report(
            BattlementErrorType type,
            BattlementErrorSource source,
            string eventName,
            string message,
            Exception? exception = null,
            string? stackTrace = null,
            string? ansiStackTrace = null,
            IReadOnlyDictionary<string, string>? fields = null,
            BattlementErrorReportingDisposition reportingDisposition =
                BattlementErrorReportingDisposition.Ignore
        )
        {
            string id = ErrorId();
            var diagnosticFields = new Dictionary<string, string>(
                fields ?? new Dictionary<string, string>()
            )
            {
                ["type"] = type.ToString(),
                ["error_id"] = id,
                ["source"] = source.ToString(),
            };
            if (exception is not null)
            {
                diagnosticFields["exception_type"] = exception.GetType().FullName ?? string.Empty;
            }

            var error = new BattlementError(
                id,
                DateTimeOffset.UtcNow,
                type,
                source,
                eventName,
                message,
                exception,
                stackTrace ?? exception?.StackTrace,
                diagnosticFields,
                logger is IBattlementLogHistory history
                    ? history.RecentRecords(MaximumRecentRecords)
                    : recent.ToArray()
            )
            {
                AnsiStackTrace = ansiStackTrace,
            };
            Log(
                new BattlementLogRecord(
                    BattlementLogSeverity.Error,
                    eventName,
                    message,
                    diagnosticFields,
                    exception,
                    error.StackTrace
                )
            );
            try
            {
                sink.Report(error);
            }
            catch (Exception sinkException)
            {
                Log(
                    new BattlementLogRecord(
                        BattlementLogSeverity.Warning,
                        "battlement.error.sink_failed",
                        "The error sink could not record a diagnostic.",
                        new Dictionary<string, string> { ["error_id"] = id },
                        sinkException
                    )
                );
            }
            if (
                reportingDisposition == BattlementErrorReportingDisposition.ReportCaughtFailure
                && reportedCaughtFailures.Add(error.Id)
            )
            {
                try
                {
                    caughtFailureReporter.Report(error);
                }
                catch (Exception reportException)
                {
                    Log(
                        new BattlementLogRecord(
                            BattlementLogSeverity.Warning,
                            "battlement.error.exception_report_failed",
                            "A caught failure could not be reported as a Unity exception.",
                            new Dictionary<string, string>
                            {
                                ["error_id"] = id,
                                ["failure_category"] = reportException.GetType().Name,
                            }
                        )
                    );
                }
            }
            ShowDevelopmentError(error);
            return error;
        }

        private static string ErrorId()
        {
            string value = Convert.ToBase64String(Guid.NewGuid().ToByteArray());
            return value.TrimEnd('=').Replace('+', '-').Replace('/', '_');
        }

        private void ShowDevelopmentError(BattlementError error)
        {
            if (
                showDevelopmentError is null
                || error.Source
                    is not BattlementErrorSource.Unity
                        and not BattlementErrorSource.Native
            )
            {
                return;
            }

            try
            {
                showDevelopmentError(error);
            }
            catch (Exception exception)
            {
                Log(
                    new BattlementLogRecord(
                        BattlementLogSeverity.Warning,
                        "battlement.development_dialog.failed",
                        "The development error dialog could not show an error.",
                        new Dictionary<string, string> { ["error_id"] = error.Id },
                        exception
                    )
                );
            }
        }
    }

    internal sealed record BattlementCapturedUnityError(
        string Condition,
        string StackTrace,
        LogType Type,
        Exception? Exception = null,
        bool IsExplicit = false
    );

    internal static class BattlementUnityErrors
    {
        private static readonly object Gate = new();
        private static readonly List<Action<BattlementCapturedUnityError>> Subscribers = new();
        private static bool installed;

        public static IDisposable Subscribe(Action<BattlementCapturedUnityError> subscriber)
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
                Exception? exception = arguments.ExceptionObject as Exception;
                Publish(
                    new BattlementCapturedUnityError(
                        exception?.Message ?? "Unhandled exception",
                        exception?.StackTrace ?? string.Empty,
                        LogType.Exception,
                        exception
                    )
                );
            };
            TaskScheduler.UnobservedTaskException += (_, arguments) =>
                Publish(
                    new BattlementCapturedUnityError(
                        arguments.Exception.Message,
                        arguments.Exception.StackTrace ?? string.Empty,
                        LogType.Exception,
                        arguments.Exception
                    )
                );
            installed = true;
        }

        private static void Receive(string condition, string stackTrace, LogType type)
        {
            if (type is not LogType.Exception and not LogType.Assert)
            {
                return;
            }

            if (
                condition.Contains(
                    nameof(BattlementCaughtFailureException),
                    StringComparison.Ordinal
                )
            )
            {
                return;
            }

            Publish(new BattlementCapturedUnityError(condition, stackTrace, type));
        }

        private static void Publish(BattlementCapturedUnityError error)
        {
            Action<BattlementCapturedUnityError>[] subscribers;
            lock (Gate)
            {
                subscribers = Subscribers.ToArray();
            }

            foreach (Action<BattlementCapturedUnityError> subscriber in subscribers)
            {
                subscriber(error);
            }
        }

        private sealed class Subscription : IDisposable
        {
            private Action<BattlementCapturedUnityError>? subscriber;

            public Subscription(Action<BattlementCapturedUnityError> subscriber) =>
                this.subscriber = subscriber;

            public void Dispose()
            {
                Action<BattlementCapturedUnityError>? current = subscriber;
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
