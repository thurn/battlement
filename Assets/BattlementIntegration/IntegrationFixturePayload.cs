#nullable enable

using System;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace Battlement.Integration
{
    /// <summary>Payload used to prove game-owned handler registration in the fixture.</summary>
    public sealed class IntegrationFixturePayload
    {
        public IntegrationFixturePayload(ObjectId objectId, float scale) =>
            (ObjectId, Scale) = (objectId, scale);

        public ObjectId ObjectId { get; }

        public float Scale { get; }
    }

    /// <summary>JSON formatter shared by the fixture handler and native engine.</summary>
    public sealed class IntegrationFixturePayloadFormatter
        : JsonConverter<IntegrationFixturePayload>
    {
        public override void WriteJson(
            JsonWriter writer,
            IntegrationFixturePayload? value,
            JsonSerializer serializer
        )
        {
            if (value is null)
            {
                throw new JsonSerializationException(
                    "An integration fixture payload cannot be null."
                );
            }

            writer.WriteStartArray();
            writer.WriteValue(value.ObjectId.Value.ToString());
            writer.WriteValue(value.Scale);
            writer.WriteEndArray();
        }

        public override IntegrationFixturePayload ReadJson(
            JsonReader reader,
            Type objectType,
            IntegrationFixturePayload? existingValue,
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
                throw new JsonSerializationException(
                    "Expected an integration fixture payload pair."
                );
            }

            return new IntegrationFixturePayload(new ObjectId(objectId), values[1]!.Value<float>());
        }
    }

    /// <summary>Applies the integration command through Battlement's public API.</summary>
    public sealed class IntegrationFixtureHandler
        : IBattlementCommandHandler<IntegrationFixturePayload>
    {
        /// <summary>Gets how many commands reached this registered handler.</summary>
        public int InvocationCount { get; private set; }

        public IBattlementCommandOperation? Execute(
            CustomCommand<IntegrationFixturePayload> command,
            BattlementCommandContext context
        )
        {
            InvocationCount++;
            if (context.Objects.TryGetObject(command.Payload.ObjectId, out var target))
            {
                target!.transform.localScale = UnityEngine.Vector3.one * command.Payload.Scale;
            }

            return null;
        }
    }
}
