#nullable enable

using System;
using System.Collections.Generic;
using System.IO;

namespace Battlement
{
    internal interface IBattlementUiEventActivation
    {
        bool TryBegin(UiEventAction action, out string? route);

        void ObserveHandled(UiEventAction action, string route);

        void Reject(string reason);
    }

    internal sealed class BattlementUiEventDispatcher
    {
        private readonly IBattlementTransport transport;
        private readonly IBattlementProtocolCodec codec;
        private readonly BattlementResponseStream responses;
        private readonly IBattlementClock clock;
        private readonly Func<ReadOnlyMemory<byte>, Response<ICommand>> decodeResponse;
        private readonly Func<bool> canDecodeInBackground;
        private readonly Action<
            string,
            BattlementUiEventTransportResult?,
            TimeSpan,
            Exception?
        > reportFailure;
        private readonly Action<BattlementLogSeverity, string, string> log;
        private readonly IBattlementUiEventActivation activation;
        private readonly List<BattlementUiEventInspection> inspections = new();
        private int dispatchDepth;
        private BattlementUiEventInspection? awaitingNativePrevention;

        internal BattlementUiEventDispatcher(
            IBattlementTransport transport,
            IBattlementProtocolCodec codec,
            BattlementResponseStream responses,
            IBattlementClock clock,
            Func<ReadOnlyMemory<byte>, Response<ICommand>> decodeResponse,
            Func<bool> canDecodeInBackground,
            Action<string, BattlementUiEventTransportResult?, TimeSpan, Exception?> reportFailure,
            Action<BattlementLogSeverity, string, string> log,
            IBattlementUiEventActivation activation
        )
        {
            this.transport = transport ?? throw new ArgumentNullException(nameof(transport));
            this.codec = codec ?? throw new ArgumentNullException(nameof(codec));
            this.responses = responses ?? throw new ArgumentNullException(nameof(responses));
            this.clock = clock ?? throw new ArgumentNullException(nameof(clock));
            this.decodeResponse =
                decodeResponse ?? throw new ArgumentNullException(nameof(decodeResponse));
            this.canDecodeInBackground =
                canDecodeInBackground
                ?? throw new ArgumentNullException(nameof(canDecodeInBackground));
            this.reportFailure =
                reportFailure ?? throw new ArgumentNullException(nameof(reportFailure));
            this.log = log ?? throw new ArgumentNullException(nameof(log));
            this.activation = activation ?? throw new ArgumentNullException(nameof(activation));
        }

        internal bool IsDispatching => dispatchDepth != 0;

        internal IReadOnlyList<BattlementUiEventInspection> Inspections => inspections;

        internal UiEventDisposition? Dispatch(UiEvent value, SessionId session)
        {
            var action = new UiEventAction(new ActionId(Guid.NewGuid()), session, value);
            string? activationRoute;
            bool beginsActivation = activation.TryBegin(action, out activationRoute);
            var inspection = new BattlementUiEventInspection(action, UiEventKindOf(value.Body));
            AddInspection(inspection);
            BattlementResponseStream.Reservation? reservation = null;
            TimeSpan started = clock.Elapsed;
            bool enteredTransport = false;
            dispatchDepth++;
            try
            {
                reservation = responses.Reserve(
                    decodeResponse,
                    decoded: response => ObserveDecodedResponse(response, inspection),
                    decodeFailed: _ => FailDeferredResponse(inspection)
                );
                inspection.AdmissionSequence = reservation.Sequence;
                byte[] message;
                using (BattlementProfiler.Serialization.Auto())
                    message = codec.SerializeUiEventAction(action);
                if (message.Length > BattlementProtocolLimits.MaximumMessageBytes)
                {
                    throw new InvalidDataException(
                        "A UI event cannot exceed "
                            + $"{BattlementProtocolLimits.MaximumMessageBytes} bytes."
                    );
                }

                BattlementUiEventTransportResult result;
                using (BattlementProfiler.Transport.Auto())
                {
                    enteredTransport = true;
                    result = transport.SubmitUiEvent(message);
                }
                if (result.Status != BattlementTransportStatus.Success)
                {
                    reservation.Release();
                    reservation = null;
                    FailInspection(
                        inspection,
                        BattlementUiEventInspectionOutcome.FailedAfterDispatch,
                        FailureReason(result.Status),
                        "reactant.event.submit_failed",
                        clock.Elapsed - started
                    );
                    reportFailure(
                        "UI event submission failed.",
                        result,
                        clock.Elapsed - started,
                        null
                    );
                    return null;
                }
                if (
                    result.Disposition != UiEventDisposition.Continue
                    && result.Disposition != UiEventDisposition.PreventDefault
                )
                {
                    FailInspection(
                        inspection,
                        BattlementUiEventInspectionOutcome.FailedAfterDispatch,
                        BattlementUiEventFailureReason.InvalidDisposition,
                        "reactant.event.invalid_disposition",
                        clock.Elapsed - started
                    );
                    throw new InvalidDataException(
                        $"UI event returned unknown disposition {(uint)result.Disposition}."
                    );
                }
                if (result.ResponsePayload.IsEmpty)
                {
                    throw new InvalidDataException("UI event returned an empty response payload.");
                }

                bool backgroundDecode =
                    codec is IBattlementBackgroundProtocolCodec
                    && canDecodeInBackground()
                    && !beginsActivation;
                reservation.Commit(result.ResponsePayload, backgroundDecode);
                reservation = null;
                inspection.Disposition = result.Disposition;
                inspection.PreventedByReactant =
                    result.Disposition == UiEventDisposition.PreventDefault
                    && !value.DefaultPrevented;
                inspection.ResponseBytes = result.ResponsePayload.Length;
                inspection.SynchronousDurationMicroseconds = Microseconds(clock.Elapsed - started);
                inspection.Outcome = BattlementUiEventInspectionOutcome.Completed;
                if (beginsActivation && activationRoute == "ui-accessibility")
                {
                    if (result.Disposition == UiEventDisposition.PreventDefault)
                    {
                        activation.ObserveHandled(action, activationRoute);
                    }
                    else
                    {
                        activation.Reject(
                            $"Semantic activation of {value.TargetId.Value} was not consumed."
                        );
                    }
                }
                awaitingNativePrevention =
                    result.Disposition == UiEventDisposition.PreventDefault ? inspection : null;
                return result.Disposition;
            }
            catch (Exception exception)
            {
                reservation?.Release();
                if (inspection.Outcome == BattlementUiEventInspectionOutcome.Pending)
                {
                    BattlementUiEventFailureReason reason = enteredTransport
                        ? CommitFailureReason(exception)
                        : RejectionFailureReason(exception);
                    FailInspection(
                        inspection,
                        enteredTransport
                            ? BattlementUiEventInspectionOutcome.FailedAfterDispatch
                            : BattlementUiEventInspectionOutcome.RejectedBeforeDispatch,
                        reason,
                        enteredTransport
                            ? "reactant.event.response_rejected"
                            : "reactant.event.submit_failed",
                        clock.Elapsed - started
                    );
                }
                reportFailure(
                    $"UI event response failed: {exception.Message}",
                    null,
                    clock.Elapsed - started,
                    exception
                );
                return null;
            }
            finally
            {
                dispatchDepth--;
            }
        }

        internal void RecordNativePrevention()
        {
            if (awaitingNativePrevention is not BattlementUiEventInspection inspection)
                return;
            inspection.NativePreventionApplied = true;
            awaitingNativePrevention = null;
            if (inspection.PreventedByReactant)
            {
                log(
                    BattlementLogSeverity.Information,
                    "reactant.event.prevented",
                    "Reactant prevented the native UI event default action."
                );
            }
        }

        internal void Clear()
        {
            inspections.Clear();
            awaitingNativePrevention = null;
        }

        private void ObserveDecodedResponse(
            Response<ICommand> response,
            BattlementUiEventInspection inspection
        )
        {
            var batchIds = new List<BatchId>();
            foreach (ResponseMessage<ICommand> message in response.Messages)
            {
                if (message is ResponseMessage<ICommand>.BatchMessage batch)
                    batchIds.Add(batch.Batch.Id);
            }
            inspection.SetBatchIds(batchIds);
            inspection.AppliedAt = clock.Elapsed;
        }

        private void FailDeferredResponse(BattlementUiEventInspection inspection) =>
            FailInspection(
                inspection,
                BattlementUiEventInspectionOutcome.DeferredApplyFailed,
                BattlementUiEventFailureReason.DeferredApply,
                "reactant.event.deferred_apply_failed",
                TimeSpan.FromTicks(checked((long)inspection.SynchronousDurationMicroseconds * 10))
            );

        private void AddInspection(BattlementUiEventInspection inspection)
        {
            const int maximumInspectionRecords = 256;
            if (inspections.Count == maximumInspectionRecords)
                inspections.RemoveAt(0);
            inspections.Add(inspection);
        }

        private void FailInspection(
            BattlementUiEventInspection inspection,
            BattlementUiEventInspectionOutcome outcome,
            BattlementUiEventFailureReason reason,
            string diagnosticCode,
            TimeSpan duration
        )
        {
            inspection.Outcome = outcome;
            inspection.FailureReason = reason;
            inspection.DiagnosticCode = diagnosticCode;
            inspection.SynchronousDurationMicroseconds = Microseconds(duration);
            log(
                BattlementLogSeverity.Error,
                diagnosticCode,
                $"Reactant UI event failed: {reason}."
            );
        }

        private static UiEventKind UiEventKindOf(UiEventBody body) =>
            Enum.Parse<UiEventKind>(body.GetType().Name);

        private static BattlementUiEventFailureReason FailureReason(
            BattlementTransportStatus status
        ) =>
            status switch
            {
                BattlementTransportStatus.EngineError => BattlementUiEventFailureReason.Engine,
                BattlementTransportStatus.Panic => BattlementUiEventFailureReason.Panic,
                _ => BattlementUiEventFailureReason.NativeTransport,
            };

        private static BattlementUiEventFailureReason RejectionFailureReason(Exception exception) =>
            exception.Message.Contains("queue more than", StringComparison.Ordinal)
                ? BattlementUiEventFailureReason.QueueItemLimit
                : BattlementUiEventFailureReason.RequestValidation;

        private static BattlementUiEventFailureReason CommitFailureReason(Exception exception)
        {
            if (exception.Message.Contains("response bytes", StringComparison.Ordinal))
                return BattlementUiEventFailureReason.QueueByteLimit;
            if (
                exception.Message.Contains(
                    "response reservation",
                    StringComparison.OrdinalIgnoreCase
                )
            )
                return BattlementUiEventFailureReason.ResponseCommitInvariant;
            return BattlementUiEventFailureReason.ResponseSerialization;
        }

        private static ulong Microseconds(TimeSpan duration) =>
            checked((ulong)Math.Max(0, duration.Ticks / 10));
    }
}
