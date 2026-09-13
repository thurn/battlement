#nullable enable

using System;

namespace Battlement.Performance
{
    internal sealed class PerformanceSmokeTransport
        : IBattlementTransport,
            IBattlementCoreMessageObserver,
            IBattlementClientMessageObserver
    {
        internal static readonly ObjectId TargetId = new(
            Guid.Parse("00000000-0000-0000-0000-000000003802")
        );

        private static readonly SessionId Session = new(
            Guid.Parse("00000000-0000-0000-0000-000000003800")
        );
        private static readonly SceneId Scene = new(
            Guid.Parse("00000000-0000-0000-0000-000000003803")
        );
        private static readonly ObjectId CameraId = new(
            Guid.Parse("00000000-0000-0000-0000-000000003801")
        );
        private static readonly SceneAddress SceneAddress = new("battlement/integration/scene");

        public int ClickCount { get; private set; }

        private Action? submittedAction;
        private UiEventAction? submittedUiEvent;

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> message) =>
            Result(
                PerformanceFlatBufferResponses.Snapshot(
                    Session.Value,
                    Scene.Value,
                    CameraId.Value,
                    TargetId.Value,
                    SceneAddress.Value
                )
            );

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> message)
        {
            if (
                submittedAction is Action action
                && action.Body is ActionBody.PointerClick click
                && click.ObjectId == TargetId
            )
            {
                ClickCount++;
                return Result(
                    PerformanceFlatBufferResponses.Tween(
                        Session.Value,
                        action.Id.Value,
                        Guid.Parse("00000000-0000-0000-0000-000000003805"),
                        Guid.Parse("00000000-0000-0000-0000-000000003804"),
                        TargetId.Value
                    )
                );
            }

            return Result(PerformanceFlatBufferResponses.Empty(Session.Value));
        }

        public BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> message)
        {
            UiEventAction action =
                submittedUiEvent
                ?? throw new InvalidOperationException("No UI event was observed.");
            return new BattlementUiEventTransportResult(
                BattlementTransportStatus.Success,
                action.Event.DefaultPrevented
                    ? UiEventDisposition.PreventDefault
                    : UiEventDisposition.Continue,
                PerformanceFlatBufferResponses.Empty(Session.Value)
            );
        }

        public BattlementTransportResult Poll() => new(BattlementTransportStatus.NoMessage);

        public void Stop() { }

        public void Dispose() { }

        public void RecordAction(Action value) => submittedAction = value;

        public void RecordConnect(Connect value) { }

        public void RecordUiEvent(UiEventAction value) => submittedUiEvent = value;

        public void RecordBatchFailure(BatchFailed<CoreErrorCode> value) { }

        public void RecordOperationFailure(OperationFailed<CoreErrorCode> value) { }

        private static BattlementTransportResult Result(ReadOnlyMemory<byte> response) =>
            new(BattlementTransportStatus.Success, response);
    }
}
