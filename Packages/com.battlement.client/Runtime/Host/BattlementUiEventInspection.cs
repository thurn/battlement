#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>Terminal progress of one synchronous UI event and its deferred response.</summary>
    public enum BattlementUiEventInspectionOutcome
    {
        Pending,
        Completed,
        StaleTarget,
        RejectedBeforeDispatch,
        FailedAfterDispatch,
        DeferredApplyFailed,
    }

    /// <summary>Specific cause of an unsuccessful UI event inspection.</summary>
    public enum BattlementUiEventFailureReason
    {
        QueueItemLimit,
        QueueByteLimit,
        SessionNotAccepting,
        RequestValidation,
        StaleSession,
        NativeTransport,
        Engine,
        ResponseSerialization,
        InvalidDisposition,
        ResponseCommitInvariant,
        Panic,
        DeferredApply,
    }

    /// <summary>One causal record spanning immediate UI handling and deferred processing.</summary>
    public sealed class BattlementUiEventInspection
    {
        private BatchId[] resultingBatchIds = Array.Empty<BatchId>();

        internal BattlementUiEventInspection(UiEventAction action, UiEventKind kind)
        {
            ActionId = action.Id;
            SessionId = action.SessionId;
            TargetId = action.Event.TargetId;
            Kind = kind;
            Cancelable = action.Event.Cancelable;
            PreventedBeforeReactant = action.Event.DefaultPrevented;
        }

        public ActionId ActionId { get; }

        public SessionId SessionId { get; }

        public ObjectId TargetId { get; }

        public UiEventKind Kind { get; }

        public bool Cancelable { get; }

        public bool PreventedBeforeReactant { get; }

        public bool PreventedByReactant { get; internal set; }

        public bool NativePreventionApplied { get; internal set; }

        public UiEventDisposition Disposition { get; internal set; }

        public ulong? AdmissionSequence { get; internal set; }

        public ulong SynchronousDurationMicroseconds { get; internal set; }

        public IReadOnlyList<BatchId> ResultingBatchIds => resultingBatchIds;

        public int ResponseBytes { get; internal set; }

        public TimeSpan? AppliedAt { get; internal set; }

        public BattlementUiEventInspectionOutcome Outcome { get; internal set; }

        public BattlementUiEventFailureReason? FailureReason { get; internal set; }

        public string? DiagnosticCode { get; internal set; }

        internal void SetBatchIds(IReadOnlyList<BatchId> values)
        {
            resultingBatchIds = new BatchId[values.Count];
            for (int index = 0; index < values.Count; index++)
                resultingBatchIds[index] = values[index];
        }
    }
}
