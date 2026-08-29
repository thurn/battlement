#nullable enable

using System;
using System.Collections.Generic;
using Battlement.Errors;

namespace Battlement
{
    internal sealed record DittoDetectedFailure(
        DittoErrorCode Code,
        DittoErrorSource Source,
        ulong RecordSequence,
        string? BattlementErrorId,
        string Message
    );

    internal sealed class DittoFunctionalErrorGate : IDisposable
    {
        private readonly BattlementLogObserver observer;
        private bool open;

        public DittoFunctionalErrorGate(BattlementLogObserver observer) =>
            this.observer = observer ?? throw new ArgumentNullException(nameof(observer));

        public void Open()
        {
            if (open)
            {
                throw new InvalidOperationException("The functional error gate is already open.");
            }
            observer.Drain();
            open = true;
        }

        public DittoDetectedFailure? Poll()
        {
            BattlementLogEntry[] entries = observer.Drain();
            if (!open)
            {
                return null;
            }
            DittoDetectedFailure? first = null;
            foreach (BattlementLogEntry entry in entries)
            {
                DittoDetectedFailure? candidate = Classify(entry);
                if (candidate is null)
                {
                    continue;
                }
                first ??= candidate;
                bool sameUnityFailure = first.Code == candidate.Code;
                if (
                    first.BattlementErrorId is null
                    && sameUnityFailure
                    && StructuredUnity(candidate)
                )
                {
                    first = first with { BattlementErrorId = candidate.BattlementErrorId };
                }
            }
            if (first is not null)
            {
                open = false;
            }
            return first;
        }

        public void Close()
        {
            observer.Drain();
            open = false;
        }

        public void Dispose() => observer.Dispose();

        private static DittoDetectedFailure? Classify(BattlementLogEntry entry)
        {
            BattlementLogRecord record = entry.Record;
            if (entry.Source == "unity")
            {
                if (record.Severity != BattlementLogSeverity.Error || CaughtEnvelope(record))
                {
                    return null;
                }
                return new DittoDetectedFailure(
                    record.EventName switch
                    {
                        "unity.assert" => DittoErrorCode.RuntimeUnityAssert,
                        "unity.exception" => DittoErrorCode.RuntimeUnityException,
                        _ => DittoErrorCode.RuntimeUnityError,
                    },
                    DittoErrorSource.Unity,
                    entry.Sequence,
                    null,
                    record.Message
                );
            }
            if (!Structured(record, out string errorId, out string type, out string source))
            {
                return null;
            }
            if (type == nameof(BattlementErrorType.Logged) && source == "Unity")
            {
                return new DittoDetectedFailure(
                    Field(record, "log_type") == "Assert"
                        ? DittoErrorCode.RuntimeUnityAssert
                        : DittoErrorCode.RuntimeUnityException,
                    DittoErrorSource.Unity,
                    entry.Sequence,
                    errorId,
                    record.Message
                );
            }
            if (type == nameof(BattlementErrorType.Logged))
            {
                return null;
            }
            bool panic = source == "Native" && Field(record, "status") == "Panic";
            return new DittoDetectedFailure(
                panic ? DittoErrorCode.RuntimePanic : DittoErrorCode.RuntimeFatal,
                source == "Native" ? DittoErrorSource.Rust : DittoErrorSource.DittoPlayer,
                entry.Sequence,
                errorId,
                record.Message
            );
        }

        private static bool CaughtEnvelope(BattlementLogRecord record) =>
            record.EventName == "unity.exception"
            && record.Message.Contains(
                nameof(BattlementCaughtFailureException),
                StringComparison.Ordinal
            );

        private static bool Structured(
            BattlementLogRecord record,
            out string errorId,
            out string type,
            out string source
        )
        {
            IReadOnlyDictionary<string, string>? fields = record.Fields;
            errorId = Field(fields, "error_id");
            type = Field(fields, "type");
            source = Field(fields, "source");
            return errorId.Length > 0 && type.Length > 0 && source.Length > 0;
        }

        private static string Field(BattlementLogRecord record, string name) =>
            Field(record.Fields, name);

        private static string Field(IReadOnlyDictionary<string, string>? fields, string name) =>
            fields is not null && fields.TryGetValue(name, out string value) ? value : string.Empty;

        private static bool StructuredUnity(DittoDetectedFailure failure) =>
            failure.Source == DittoErrorSource.Unity && failure.BattlementErrorId is not null;
    }
}
