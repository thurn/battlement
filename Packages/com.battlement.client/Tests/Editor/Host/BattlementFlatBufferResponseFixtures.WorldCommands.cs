#nullable enable

using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Payload ExternalUrl(
            FlatBufferBuilder builder,
            CommandBody.ApplicationOpenUrl value
        ) =>
            new(
                Wire.CoreCommandKind.ApplicationOpenUrl,
                Wire.CoreCommandPayload.ExternalUrlPayload,
                Wire.ExternalUrlPayload.CreateExternalUrlPayload(
                    builder,
                    builder.CreateString(value.Url)
                ).Value
            );

        private static Payload DebugUi(FlatBufferBuilder builder, CommandBody.DebugUi value) =>
            new(
                Wire.CoreCommandKind.DebugUi,
                Wire.CoreCommandPayload.DebugUiPayload,
                Wire.DebugUiPayload.CreateDebugUiPayload(
                    builder,
                    (Wire.DebugUiSurface)value.Surface,
                    value.Visible
                ).Value
            );

        private static Payload ReplaceAssets(
            FlatBufferBuilder builder,
            CommandBody.Assets.ReplaceSet value
        )
        {
            var assets = new int[value.PreparedAssets.Count];
            for (int index = 0; index < assets.Length; index++)
                assets[index] = WriteAsset(builder, value.PreparedAssets[index]).Value;
            return new(
                Wire.CoreCommandKind.AssetsReplaceSet,
                Wire.CoreCommandPayload.ReplaceAssetSetPayload,
                Wire.ReplaceAssetSetPayload.CreateReplaceAssetSetPayload(
                    builder,
                    OffsetVector(builder, assets)
                ).Value
            );
        }

        private static Payload LoadScene(FlatBufferBuilder builder, CommandBody.Scene.Load value)
        {
            StringOffset address = builder.CreateString(value.Address.Value);
            Wire.SceneLoadPayload.StartSceneLoadPayload(builder);
            Wire.SceneLoadPayload.AddMakePrimary(builder, value.MakePrimary);
            Wire.SceneLoadPayload.AddAddress(builder, address);
            Wire.SceneLoadPayload.AddSceneId(builder, Uuid(builder, value.SceneId.Value));
            return new(
                Wire.CoreCommandKind.SceneLoad,
                Wire.CoreCommandPayload.SceneLoadPayload,
                Wire.SceneLoadPayload.EndSceneLoadPayload(builder).Value
            );
        }

        private static Payload SceneId(
            FlatBufferBuilder builder,
            SceneId value,
            Wire.CoreCommandKind kind
        )
        {
            Wire.SceneIdPayload.StartSceneIdPayload(builder);
            Wire.SceneIdPayload.AddSceneId(builder, Uuid(builder, value.Value));
            return new(
                kind,
                Wire.CoreCommandPayload.SceneIdPayload,
                Wire.SceneIdPayload.EndSceneIdPayload(builder).Value
            );
        }

        private static Payload CreateObject(
            FlatBufferBuilder builder,
            CommandBody.Object.Create value
        ) =>
            new(
                Wire.CoreCommandKind.ObjectCreate,
                Wire.CoreCommandPayload.ObjectCreatePayload,
                Wire.ObjectCreatePayload.CreateObjectCreatePayload(
                    builder,
                    WriteObject(builder, value.GameObject)
                ).Value
            );

        private static Payload ObjectId(
            FlatBufferBuilder builder,
            Battlement.ObjectId value,
            Wire.CoreCommandKind kind
        )
        {
            Wire.ObjectIdPayload.StartObjectIdPayload(builder);
            Wire.ObjectIdPayload.AddObjectId(builder, Uuid(builder, value.Value));
            return new(
                kind,
                Wire.CoreCommandPayload.ObjectIdPayload,
                Wire.ObjectIdPayload.EndObjectIdPayload(builder).Value
            );
        }

        private static Payload ObjectEnabled(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            bool enabled,
            Wire.CoreCommandKind kind,
            Wire.CoreCommandPayload payloadType
        )
        {
            if (payloadType == Wire.CoreCommandPayload.ObjectSetActivePayload)
            {
                Wire.ObjectSetActivePayload.StartObjectSetActivePayload(builder);
                Wire.ObjectSetActivePayload.AddActive(builder, enabled);
                Wire.ObjectSetActivePayload.AddObjectId(builder, Uuid(builder, objectId.Value));
                return new(
                    kind,
                    payloadType,
                    Wire.ObjectSetActivePayload.EndObjectSetActivePayload(builder).Value
                );
            }
            Wire.ObjectEnabledPayload.StartObjectEnabledPayload(builder);
            Wire.ObjectEnabledPayload.AddEnabled(builder, enabled);
            Wire.ObjectEnabledPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            return new(
                kind,
                payloadType,
                Wire.ObjectEnabledPayload.EndObjectEnabledPayload(builder).Value
            );
        }

        private static Payload ReparentObject(
            FlatBufferBuilder builder,
            CommandBody.Object.Reparent value
        )
        {
            Wire.ObjectReparentPayload.StartObjectReparentPayload(builder);
            Wire.ObjectReparentPayload.AddWorldPositionStays(builder, value.WorldPositionStays);
            if (value.ParentId is Battlement.ObjectId parent)
                Wire.ObjectReparentPayload.AddParentId(builder, Uuid(builder, parent.Value));
            Wire.ObjectReparentPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            return new(
                Wire.CoreCommandKind.ObjectReparent,
                Wire.CoreCommandPayload.ObjectReparentPayload,
                Wire.ObjectReparentPayload.EndObjectReparentPayload(builder).Value
            );
        }

        private static Payload Position(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Vector3 value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.PositionPayload.StartPositionPayload(builder);
            Wire.PositionPayload.AddPosition(builder, Vector3(builder, value));
            Wire.PositionPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.PositionPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.PositionPayload,
                Wire.PositionPayload.EndPositionPayload(builder).Value
            );
        }

        private static Payload TweenPosition(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Vector3 value,
            Tween tween,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.TweenPositionPayload.StartTweenPositionPayload(builder);
            Wire.TweenPositionPayload.AddTween(builder, Tween(builder, tween));
            Wire.TweenPositionPayload.AddPosition(builder, Vector3(builder, value));
            Wire.TweenPositionPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TweenPositionPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.TweenPositionPayload,
                Wire.TweenPositionPayload.EndTweenPositionPayload(builder).Value
            );
        }

        private static Payload Rotation(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Quaternion value,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.RotationPayload.StartRotationPayload(builder);
            Wire.RotationPayload.AddRotation(builder, Quaternion(builder, value));
            Wire.RotationPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.RotationPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.RotationPayload,
                Wire.RotationPayload.EndRotationPayload(builder).Value
            );
        }

        private static Payload TweenRotation(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Quaternion value,
            Tween tween,
            ConflictPolicy conflict,
            Wire.CoreCommandKind kind
        )
        {
            Wire.TweenRotationPayload.StartTweenRotationPayload(builder);
            Wire.TweenRotationPayload.AddTween(builder, Tween(builder, tween));
            Wire.TweenRotationPayload.AddRotation(builder, Quaternion(builder, value));
            Wire.TweenRotationPayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TweenRotationPayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                kind,
                Wire.CoreCommandPayload.TweenRotationPayload,
                Wire.TweenRotationPayload.EndTweenRotationPayload(builder).Value
            );
        }

        private static Payload Scale(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Vector3 value,
            ConflictPolicy conflict
        )
        {
            Wire.ScalePayload.StartScalePayload(builder);
            Wire.ScalePayload.AddScale(builder, Vector3(builder, value));
            Wire.ScalePayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.ScalePayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                Wire.CoreCommandKind.TransformSetLocalScale,
                Wire.CoreCommandPayload.ScalePayload,
                Wire.ScalePayload.EndScalePayload(builder).Value
            );
        }

        private static Payload TweenScale(
            FlatBufferBuilder builder,
            Battlement.ObjectId objectId,
            Vector3 value,
            Tween tween,
            ConflictPolicy conflict
        )
        {
            Wire.TweenScalePayload.StartTweenScalePayload(builder);
            Wire.TweenScalePayload.AddTween(builder, Tween(builder, tween));
            Wire.TweenScalePayload.AddScale(builder, Vector3(builder, value));
            Wire.TweenScalePayload.AddObjectId(builder, Uuid(builder, objectId.Value));
            Wire.TweenScalePayload.AddOnConflict(builder, Conflict(conflict));
            return new(
                Wire.CoreCommandKind.TransformTweenLocalScale,
                Wire.CoreCommandPayload.TweenScalePayload,
                Wire.TweenScalePayload.EndTweenScalePayload(builder).Value
            );
        }

        private static Payload SetMaterial(
            FlatBufferBuilder builder,
            CommandBody.Renderer.SetMaterial value
        )
        {
            StringOffset address = builder.CreateString(value.Address.Value);
            Wire.SetMaterialPayload.StartSetMaterialPayload(builder);
            Wire.SetMaterialPayload.AddSlot(builder, value.Slot);
            Wire.SetMaterialPayload.AddAddress(builder, address);
            Wire.SetMaterialPayload.AddObjectId(builder, Uuid(builder, value.ObjectId.Value));
            Wire.SetMaterialPayload.AddOnConflict(builder, Conflict(value.OnConflict));
            return new(
                Wire.CoreCommandKind.RendererSetMaterial,
                Wire.CoreCommandPayload.SetMaterialPayload,
                Wire.SetMaterialPayload.EndSetMaterialPayload(builder).Value
            );
        }

        private static Offset<Wire.Vector3d> Vector3(FlatBufferBuilder builder, Vector3 value) =>
            Wire.Vector3d.CreateVector3d(builder, value.X, value.Y, value.Z);

        private static Offset<Wire.Quaterniond> Quaternion(
            FlatBufferBuilder builder,
            Quaternion value
        ) => Wire.Quaterniond.CreateQuaterniond(builder, value.X, value.Y, value.Z, value.W);
    }
}
