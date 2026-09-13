#nullable enable

using System;
using UnityEngine;
using Wire = Battlement.FlatBuffers.FixtureGenerated;

namespace Battlement.CustomFixtures
{
    public sealed class FlashPayload
    {
        public FlashPayload(ObjectId objectId, float scale) =>
            (ObjectId, Scale) = (objectId, scale);

        public ObjectId ObjectId { get; }

        public float Scale { get; }
    }

    public enum FixtureError
    {
        Rejected,
        Delayed,
    }

    public enum FixtureHandlerMode
    {
        Complete,
        Throw,
        Reject,
        Track,
        EmitNestedAction,
        EmitNestedActionAndReject,
    }

    public sealed class FixtureHandler : IBattlementFlatBufferCommandHandler<Wire.FlashPayload>
    {
        private readonly BattlementRunner? runner;

        public FixtureHandler(
            FixtureHandlerMode mode = FixtureHandlerMode.Complete,
            BattlementRunner? runner = null
        ) => (Mode, this.runner) = (mode, runner);

        public FixtureHandlerMode Mode { get; set; }

        public int InvocationCount { get; private set; }

        public int InvocationThreadId { get; private set; }

        public BattlementCommandContext? LastContext { get; private set; }

        public FixtureOperation? Operation { get; private set; }

        public Wire.FlashPayload? LastFlatBufferPayload { get; private set; }

        public IBattlementCommandOperation? Execute(
            BattlementFlatBufferCommand<Wire.FlashPayload> command,
            BattlementCommandContext context
        )
        {
            LastFlatBufferPayload = command.Payload;
            return Execute(
                new ObjectId(
                    BattlementFlatBufferCore.ReadUuid(command.Payload.ObjectId, "flash object")
                ),
                command.Payload.Scale,
                context
            );
        }

        private IBattlementCommandOperation? Execute(
            ObjectId objectId,
            float scale,
            BattlementCommandContext context
        )
        {
            InvocationCount++;
            InvocationThreadId = Environment.CurrentManagedThreadId;
            LastContext = context;
            switch (Mode)
            {
                case FixtureHandlerMode.Complete:
                    break;
                case FixtureHandlerMode.Throw:
                    throw new InvalidOperationException("fixture handler exploded");
                case FixtureHandlerMode.Reject:
                    throw new BattlementCommandFailureException<FixtureError>(
                        FixtureError.Rejected,
                        "fixture command rejected"
                    );
                case FixtureHandlerMode.Track:
                    Operation = new FixtureOperation(context);
                    return context.ForObject(objectId, Operation, controlsTransform: true);
                case FixtureHandlerMode.EmitNestedAction:
                    runner!.EmitCustomAction(
                        "fixture.flash.completed",
                        new FlashPayload(objectId, scale)
                    );
                    break;
                case FixtureHandlerMode.EmitNestedActionAndReject:
                    runner!.EmitCustomAction(
                        "fixture.flash.completed",
                        new FlashPayload(objectId, scale)
                    );
                    throw new BattlementCommandFailureException<FixtureError>(
                        FixtureError.Rejected,
                        "fixture command rejected"
                    );
                default:
                    throw new ArgumentOutOfRangeException(nameof(Mode));
            }

            if (context.Objects.TryGetObject(objectId, out GameObject? target))
            {
                target!.transform.localScale = UnityEngine.Vector3.one * scale;
            }

            return null;
        }
    }

    public sealed class FixtureOperation : IBattlementCommandOperation
    {
        private readonly BattlementCommandContext context;

        public FixtureOperation(BattlementCommandContext context) => this.context = context;

        public bool IsInfinite => false;

        public bool IsCompleteNow { get; set; }

        public bool ShouldFail { get; set; }

        public bool WasCancelled { get; private set; }

        public bool CancellationWasRequested { get; private set; }

        public bool IsComplete(TimeSpan now)
        {
            if (ShouldFail)
            {
                throw new BattlementCommandFailureException<FixtureError>(
                    FixtureError.Delayed,
                    "fixture operation failed"
                );
            }

            return IsCompleteNow;
        }

        public void Cancel()
        {
            WasCancelled = true;
            CancellationWasRequested = context.Cancellation.IsCancellationRequested;
        }
    }
}
