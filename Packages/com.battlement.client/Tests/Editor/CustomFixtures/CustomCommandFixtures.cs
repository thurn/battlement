#nullable enable

using System;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using UnityEngine;

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
    }

    public sealed class FlashPayloadFormatter : JsonConverter<FlashPayload>
    {
        public override void WriteJson(
            JsonWriter writer,
            FlashPayload? value,
            JsonSerializer serializer
        )
        {
            if (value is null)
            {
                throw new JsonSerializationException("A flash payload cannot be null.");
            }

            writer.WriteStartArray();
            writer.WriteValue(value.ObjectId.Value.ToString());
            writer.WriteValue(value.Scale);
            writer.WriteEndArray();
        }

        public override FlashPayload ReadJson(
            JsonReader reader,
            Type objectType,
            FlashPayload? existingValue,
            bool hasExistingValue,
            JsonSerializer serializer
        )
        {
            JArray values = JArray.Load(reader);
            if (
                values.Count != 2
                || !Guid.TryParse(values[0]?.Value<string>(), out Guid objectId)
                || objectId == Guid.Empty
            )
            {
                throw new JsonSerializationException("Expected a flash payload pair.");
            }

            return new FlashPayload(new ObjectId(objectId), values[1]!.Value<float>());
        }
    }

    public sealed class FixtureErrorFormatter : JsonConverter<FixtureError>
    {
        public override void WriteJson(
            JsonWriter writer,
            FixtureError value,
            JsonSerializer serializer
        ) => writer.WriteValue(value.ToString());

        public override FixtureError ReadJson(
            JsonReader reader,
            Type objectType,
            FixtureError existingValue,
            bool hasExistingValue,
            JsonSerializer serializer
        ) => Enum.Parse<FixtureError>(reader.Value?.ToString() ?? string.Empty);
    }

    public sealed class RejectingFlashPayloadFormatter : JsonConverter<FlashPayload>
    {
        public override void WriteJson(
            JsonWriter writer,
            FlashPayload? value,
            JsonSerializer serializer
        ) => new FlashPayloadFormatter().WriteJson(writer, value, serializer);

        public override FlashPayload ReadJson(
            JsonReader reader,
            Type objectType,
            FlashPayload? existingValue,
            bool hasExistingValue,
            JsonSerializer serializer
        ) => throw new JsonSerializationException("fixture payload rejected");
    }

    public sealed class FixtureHandler : IBattlementCommandHandler<FlashPayload>
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

        public IBattlementCommandOperation? Execute(
            CustomCommand<FlashPayload> command,
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
