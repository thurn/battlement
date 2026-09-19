#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload GeometryObservation(
            FlatBufferBuilder builder,
            CommandBody.GeometryObservation command
        )
        {
            GeometryObservationUpdate value = command.Value;
            var added = new int[value.Added.Count];
            for (int index = 0; index < added.Length; index++)
            {
                GeometryObservation observation = value.Added[index];
                Offset<Wire.GeometryObservationTarget> target = WriteGeometryTarget(
                    builder,
                    observation.Target
                );
                Wire.GeometryObservation.StartGeometryObservation(builder);
                Wire.GeometryObservation.AddTarget(builder, target);
                Wire.GeometryObservation.AddObservationId(
                    builder,
                    Uuid(builder, observation.ObservationId.Value)
                );
                added[index] = Wire.GeometryObservation.EndGeometryObservation(builder).Value;
            }
            VectorOffset addedVector = OffsetVector(builder, added);
            builder.StartVector(16, value.Removed.Count, 1);
            for (int index = value.Removed.Count - 1; index >= 0; index--)
                Uuid(builder, value.Removed[index].Value);
            VectorOffset removedVector = builder.EndVector();
            Offset<Wire.GeometryObservationUpdate> payload =
                Wire.GeometryObservationUpdate.CreateGeometryObservationUpdate(
                    builder,
                    addedVector,
                    removedVector
                );
            return new Payload(
                Wire.CoreCommandKind.GeometryObservationUpdate,
                Wire.CoreCommandPayload.GeometryObservationUpdate,
                payload.Value
            );
        }

        private static Offset<Wire.GeometryObservationTarget> WriteGeometryTarget(
            FlatBufferBuilder builder,
            GeometryObservationTarget value
        )
        {
            StringOffset? anchor = value is GeometryObservationTarget.WorldAnchor worldAnchor
                ? builder.CreateString(worldAnchor.Anchor.Value)
                : null;
            Wire.GeometryObservationTarget.StartGeometryObservationTarget(builder);
            if (anchor is StringOffset anchorValue)
                Wire.GeometryObservationTarget.AddAnchor(builder, anchorValue);
            switch (value)
            {
                case GeometryObservationTarget.UiElement element:
                    Wire.GeometryObservationTarget.AddObjectId(
                        builder,
                        Uuid(builder, element.ObjectId.Value)
                    );
                    Wire.GeometryObservationTarget.AddKind(
                        builder,
                        Wire.GeometryTargetKind.UiElement
                    );
                    break;
                case GeometryObservationTarget.Viewport viewport:
                    Wire.GeometryObservationTarget.AddDisplayId(builder, viewport.DisplayId.Value);
                    Wire.GeometryObservationTarget.AddKind(
                        builder,
                        Wire.GeometryTargetKind.Viewport
                    );
                    break;
                case GeometryObservationTarget.WorldOrigin origin:
                    AddGeometryWorldTarget(
                        builder,
                        origin.ObjectId,
                        origin.Camera,
                        Wire.GeometryTargetKind.WorldOrigin
                    );
                    break;
                case GeometryObservationTarget.WorldAnchor anchored:
                    AddGeometryWorldTarget(
                        builder,
                        anchored.ObjectId,
                        anchored.Camera,
                        Wire.GeometryTargetKind.WorldAnchor
                    );
                    break;
                case GeometryObservationTarget.WorldRenderedBounds bounds:
                    AddGeometryWorldTarget(
                        builder,
                        bounds.ObjectId,
                        bounds.Camera,
                        Wire.GeometryTargetKind.WorldRenderedBounds
                    );
                    break;
                case GeometryObservationTarget.WorldRestBounds bounds:
                    Wire.GeometryObservationTarget.AddRequestId(
                        builder,
                        Uuid(builder, bounds.RequestId.Value)
                    );
                    Wire.GeometryObservationTarget.AddObjectId(
                        builder,
                        Uuid(builder, bounds.ObjectId.Value)
                    );
                    Wire.GeometryObservationTarget.AddKind(
                        builder,
                        Wire.GeometryTargetKind.WorldRestBounds
                    );
                    break;
                case GeometryObservationTarget.PresentationWork:
                    Wire.GeometryObservationTarget.AddKind(
                        builder,
                        Wire.GeometryTargetKind.PresentationWork
                    );
                    break;
                default:
                    throw Unsupported(value);
            }
            return Wire.GeometryObservationTarget.EndGeometryObservationTarget(builder);
        }

        private static void AddGeometryWorldTarget(
            FlatBufferBuilder builder,
            ObjectId objectId,
            CameraTarget camera,
            Wire.GeometryTargetKind kind
        )
        {
            switch (camera)
            {
                case CameraTarget.Input:
                    Wire.GeometryObservationTarget.AddCameraKind(
                        builder,
                        Wire.CameraTargetKind.Input
                    );
                    break;
                case CameraTarget.Object value:
                    Wire.GeometryObservationTarget.AddCameraObjectId(
                        builder,
                        Uuid(builder, value.ObjectId.Value)
                    );
                    Wire.GeometryObservationTarget.AddCameraKind(
                        builder,
                        Wire.CameraTargetKind.Object
                    );
                    break;
                default:
                    throw Unsupported(camera);
            }
            Wire.GeometryObservationTarget.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.GeometryObservationTarget.AddKind(builder, kind);
        }
    }
}
