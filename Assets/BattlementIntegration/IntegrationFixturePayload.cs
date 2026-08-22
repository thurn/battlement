#nullable enable

using System;
using System.Buffers;
using MessagePack;
using MessagePack.Formatters;

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

    /// <summary>MessagePack formatter shared by the fixture handler and native engine.</summary>
    public sealed class IntegrationFixturePayloadFormatter
        : IMessagePackFormatter<IntegrationFixturePayload>
    {
        public void Serialize(
            ref MessagePackWriter writer,
            IntegrationFixturePayload value,
            MessagePackSerializerOptions options
        )
        {
            writer.WriteArrayHeader(2);
            writer.Write(value.ObjectId.Value.ToByteArray());
            writer.Write(value.Scale);
        }

        public IntegrationFixturePayload Deserialize(
            ref MessagePackReader reader,
            MessagePackSerializerOptions options
        )
        {
            if (reader.ReadArrayHeader() != 2)
            {
                throw new MessagePackSerializationException(
                    "Expected an integration fixture payload pair."
                );
            }

            byte[]? objectId = reader.ReadBytes()?.ToArray();
            if (objectId is null || objectId.Length != 16)
            {
                throw new MessagePackSerializationException("Expected a 16-byte object UUID.");
            }

            return new IntegrationFixturePayload(
                new ObjectId(new Guid(objectId)),
                reader.ReadSingle()
            );
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
