#nullable enable

using System;
using System.Buffers;
using MessagePack;
using MessagePack.Formatters;
using UnityEngine;

namespace Masonry.CustomFixtures
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
    }

    public sealed class FlashPayloadFormatter : IMessagePackFormatter<FlashPayload>
    {
        public void Serialize(
            ref MessagePackWriter writer,
            FlashPayload value,
            MessagePackSerializerOptions options
        )
        {
            writer.WriteArrayHeader(2);
            writer.Write(value.ObjectId.Value.ToByteArray());
            writer.Write(value.Scale);
        }

        public FlashPayload Deserialize(
            ref MessagePackReader reader,
            MessagePackSerializerOptions options
        )
        {
            if (reader.ReadArrayHeader() != 2)
            {
                throw new MessagePackSerializationException("Expected a flash payload pair.");
            }

            byte[]? objectId = reader.ReadBytes()?.ToArray();
            if (objectId is null || objectId.Length != 16)
            {
                throw new MessagePackSerializationException("Expected a 16-byte object UUID.");
            }

            return new FlashPayload(new ObjectId(new Guid(objectId)), reader.ReadSingle());
        }
    }

    public sealed class FixtureErrorFormatter : IMessagePackFormatter<FixtureError>
    {
        public void Serialize(
            ref MessagePackWriter writer,
            FixtureError value,
            MessagePackSerializerOptions options
        ) => writer.Write(value.ToString());

        public FixtureError Deserialize(
            ref MessagePackReader reader,
            MessagePackSerializerOptions options
        ) => Enum.Parse<FixtureError>(reader.ReadString() ?? string.Empty);
    }

    public sealed class RejectingFlashPayloadFormatter : IMessagePackFormatter<FlashPayload>
    {
        public void Serialize(
            ref MessagePackWriter writer,
            FlashPayload value,
            MessagePackSerializerOptions options
        ) => new FlashPayloadFormatter().Serialize(ref writer, value, options);

        public FlashPayload Deserialize(
            ref MessagePackReader reader,
            MessagePackSerializerOptions options
        ) => throw new MessagePackSerializationException("fixture payload rejected");
    }

    public sealed class FixtureHandler : IMasonryCommandHandler<FlashPayload>
    {
        private readonly MasonryRunner? runner;

        public FixtureHandler(
            FixtureHandlerMode mode = FixtureHandlerMode.Complete,
            MasonryRunner? runner = null
        ) => (Mode, this.runner) = (mode, runner);

        public FixtureHandlerMode Mode { get; set; }

        public int InvocationCount { get; private set; }

        public int InvocationThreadId { get; private set; }

        public MasonryCommandContext? LastContext { get; private set; }

        public FixtureOperation? Operation { get; private set; }

        public IMasonryCommandOperation? Execute(
            CustomCommand<FlashPayload> command,
            MasonryCommandContext context
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
                    throw new MasonryCommandFailureException<FixtureError>(
                        FixtureError.Rejected,
                        "fixture command rejected"
                    );
                case FixtureHandlerMode.Track:
                    Operation = new FixtureOperation(context);
                    return context.ForObject(
                        command.Payload.ObjectId,
                        Operation,
                        controlsTransform: true
                    );
                case FixtureHandlerMode.EmitNestedAction:
                    runner!.EmitCustomAction(
                        "fixture.flash.completed",
                        command.Payload,
                        new FlashPayloadFormatter()
                    );
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(Mode));
            }

            if (context.Objects.TryGetObject(command.Payload.ObjectId, out GameObject? target))
            {
                target!.transform.localScale = UnityEngine.Vector3.one * command.Payload.Scale;
            }

            return null;
        }
    }

    public sealed class FixtureOperation : IMasonryCommandOperation
    {
        private readonly MasonryCommandContext context;

        public FixtureOperation(MasonryCommandContext context) => this.context = context;

        public bool IsInfinite => false;

        public bool IsCompleteNow { get; set; }

        public bool ShouldFail { get; set; }

        public bool WasCancelled { get; private set; }

        public bool CancellationWasRequested { get; private set; }

        public bool IsComplete(TimeSpan now)
        {
            if (ShouldFail)
            {
                throw new MasonryCommandFailureException<FixtureError>(
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
