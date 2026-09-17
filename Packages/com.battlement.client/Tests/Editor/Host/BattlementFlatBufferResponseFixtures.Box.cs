#nullable enable
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Offset<Wire.BoxHitRegionObject> WriteBox(
            FlatBufferBuilder builder,
            BoxHitRegionState state
        )
        {
            Wire.BoxHitRegionObject.StartBoxHitRegionObject(builder);
            Wire.BoxHitRegionObject.AddSize(
                builder,
                Wire.Vector3d.CreateVector3d(builder, state.Size.X, state.Size.Y, state.Size.Z)
            );
            Wire.BoxHitRegionObject.AddCenter(
                builder,
                Wire.Vector3d.CreateVector3d(
                    builder,
                    state.Center.X,
                    state.Center.Y,
                    state.Center.Z
                )
            );
            return Wire.BoxHitRegionObject.EndBoxHitRegionObject(builder);
        }

        private static Payload SetBoxHitRegion(
            FlatBufferBuilder builder,
            CommandBody.SetBoxHitRegion value
        )
        {
            var region = WriteBox(builder, value.Region);
            Wire.BoxHitRegionPayload.StartBoxHitRegionPayload(builder);
            Wire.BoxHitRegionPayload.AddRegion(builder, region);
            Wire.BoxHitRegionPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new Payload(
                Wire.CoreCommandKind.BoxHitRegionSetGeometry,
                Wire.CoreCommandPayload.BoxHitRegionPayload,
                Wire.BoxHitRegionPayload.EndBoxHitRegionPayload(builder).Value
            );
        }
    }
}
