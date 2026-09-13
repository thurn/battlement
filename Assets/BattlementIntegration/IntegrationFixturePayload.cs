#nullable enable

using Wire = Battlement.FlatBuffers.FixtureGenerated;

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

    /// <summary>Applies the integration command through Battlement's public API.</summary>
    public sealed class IntegrationFixtureHandler
        : IBattlementFlatBufferCommandHandler<Wire.FlashPayload>
    {
        /// <summary>Gets how many commands reached this registered handler.</summary>
        public int InvocationCount { get; private set; }

        public IBattlementCommandOperation? Execute(
            BattlementFlatBufferCommand<Wire.FlashPayload> command,
            BattlementCommandContext context
        )
        {
            InvocationCount++;
            var objectId = new ObjectId(
                BattlementFlatBufferCore.ReadUuid(command.Payload.ObjectId, "flash object")
            );
            if (context.Objects.TryGetObject(objectId, out var target))
            {
                target!.transform.localScale = UnityEngine.Vector3.one * command.Payload.Scale;
            }

            return null;
        }
    }
}
