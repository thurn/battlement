#nullable enable

using System;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Performance
{
    internal static class PerformanceFlatBufferResponses
    {
        internal static ReadOnlyMemory<byte> Empty(Guid sessionId)
        {
            var builder = new FlatBufferBuilder(256);
            VectorOffset messages = Wire.Response.CreateMessagesVector(
                builder,
                Array.Empty<Offset<Wire.ResponseMessageEntry>>()
            );
            Wire.Response.StartResponse(builder);
            Wire.Response.AddMessages(builder, messages);
            Wire.Response.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, sessionId)
            );
            Offset<Wire.Response> response = Wire.Response.EndResponse(builder);
            Wire.Response.FinishSizePrefixedResponseBuffer(builder, response);
            return builder.SizedByteArray();
        }

        internal static ReadOnlyMemory<byte> Snapshot(
            Guid sessionId,
            Guid sceneId,
            Guid cameraId,
            Guid targetId,
            string sceneAddress
        )
        {
            var builder = new FlatBufferBuilder(2048);
            StringOffset address = builder.CreateString(sceneAddress);
            Offset<Wire.PreparedAsset> asset = Wire.PreparedAsset.CreatePreparedAsset(
                builder,
                Wire.PreparedAssetKind.Scene,
                address
            );
            Wire.Scene.StartScene(builder);
            Wire.Scene.AddAddress(builder, address);
            Wire.Scene.AddSceneId(builder, BattlementFlatBufferWriter.WriteUuid(builder, sceneId));
            Offset<Wire.Scene> scene = Wire.Scene.EndScene(builder);
            Offset<Wire.GameObject> camera = Camera(builder, cameraId);
            Offset<Wire.GameObject> target = Target(builder, targetId, sceneId);
            VectorOffset assets = Wire.Snapshot.CreatePreparedAssetsVector(
                builder,
                new[] { asset }
            );
            VectorOffset scenes = Wire.Snapshot.CreateScenesVector(builder, new[] { scene });
            VectorOffset objects = Wire.Snapshot.CreateObjectsVector(
                builder,
                new[] { camera, target }
            );
            VectorOffset ui = Wire.Snapshot.CreateUiVector(
                builder,
                Array.Empty<Offset<Wire.UiDocument>>()
            );
            VectorOffset keys = Wire.Snapshot.CreateGlobalKeysVector(
                builder,
                Array.Empty<Wire.PhysicalKey>()
            );
            Offset<Wire.PanelInputConfiguration> panelInput =
                Wire.PanelInputConfiguration.CreatePanelInputConfiguration(builder);
            Wire.Snapshot.StartSnapshot(builder);
            Wire.Snapshot.AddPreparedAssets(builder, assets);
            Wire.Snapshot.AddScenes(builder, scenes);
            Wire.Snapshot.AddObjects(builder, objects);
            Wire.Snapshot.AddUi(builder, ui);
            Wire.Snapshot.AddPanelInputConfiguration(builder, panelInput);
            Wire.Snapshot.AddGlobalKeys(builder, keys);
            Wire.Snapshot.AddInputCameraId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, cameraId)
            );
            Wire.Snapshot.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, sessionId)
            );
            Offset<Wire.Snapshot> snapshot = Wire.Snapshot.EndSnapshot(builder);
            Offset<Wire.ResponseMessageEntry> entry =
                Wire.ResponseMessageEntry.CreateResponseMessageEntry(
                    builder,
                    Wire.ResponseMessage.Snapshot,
                    snapshot.Value
                );
            return Finish(builder, sessionId, entry);
        }

        internal static ReadOnlyMemory<byte> Tween(
            Guid sessionId,
            Guid actionId,
            Guid batchId,
            Guid commandId,
            Guid targetId
        )
        {
            var builder = new FlatBufferBuilder(1024);
            Wire.TweenPositionPayload.StartTweenPositionPayload(builder);
            Wire.TweenPositionPayload.AddTween(
                builder,
                Wire.Tween.CreateTween(
                    builder,
                    0,
                    500,
                    Wire.Easing.Linear,
                    Wire.TweenRepeatKind.Once,
                    0,
                    Wire.RepeatMode.Restart
                )
            );
            Wire.TweenPositionPayload.AddPosition(
                builder,
                Wire.Vector3d.CreateVector3d(builder, 2, 0, 0)
            );
            Wire.TweenPositionPayload.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, targetId)
            );
            Offset<Wire.TweenPositionPayload> payload =
                Wire.TweenPositionPayload.EndTweenPositionPayload(builder);
            Wire.CoreCommand.StartCoreCommand(builder);
            Wire.CoreCommand.AddPayload(builder, payload.Value);
            Wire.CoreCommand.AddPayloadType(builder, Wire.CoreCommandPayload.TweenPositionPayload);
            Wire.CoreCommand.AddKind(builder, Wire.CoreCommandKind.TransformTweenLocalPosition);
            Wire.CoreCommand.AddCommandId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, commandId)
            );
            Offset<Wire.CoreCommand> command = Wire.CoreCommand.EndCoreCommand(builder);
            Offset<Wire.CommandEntry> commandEntry = Wire.CommandEntry.CreateCommandEntry(
                builder,
                Wire.CommandEntryPayload.CoreCommand,
                command.Value
            );
            VectorOffset commands = Wire.ParallelCommandGroup.CreateCommandsVector(
                builder,
                new[] { commandEntry }
            );
            Offset<Wire.ParallelCommandGroup> group =
                Wire.ParallelCommandGroup.CreateParallelCommandGroup(builder, commands);
            VectorOffset groups = Wire.Batch.CreateGroupsVector(builder, new[] { group });
            Wire.Batch.StartBatch(builder);
            Wire.Batch.AddGroups(builder, groups);
            Wire.Batch.AddCausedByActionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, actionId)
            );
            Wire.Batch.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, sessionId)
            );
            Wire.Batch.AddBatchId(builder, BattlementFlatBufferWriter.WriteUuid(builder, batchId));
            Offset<Wire.Batch> batch = Wire.Batch.EndBatch(builder);
            Offset<Wire.ResponseMessageEntry> entry =
                Wire.ResponseMessageEntry.CreateResponseMessageEntry(
                    builder,
                    Wire.ResponseMessage.Batch,
                    batch.Value
                );
            return Finish(builder, sessionId, entry);
        }

        private static Offset<Wire.GameObject> Camera(FlatBufferBuilder builder, Guid cameraId)
        {
            Wire.CameraObject.StartCameraObject(builder);
            Wire.CameraObject.AddClearColor(
                builder,
                Wire.RgbaColor.CreateRgbaColor(builder, 0, 0, 0, 0)
            );
            Wire.CameraObject.AddOrthographicSize(builder, 3);
            Wire.CameraObject.AddProjection(builder, Wire.CameraProjection.Orthographic);
            Offset<Wire.CameraObject> content = Wire.CameraObject.EndCameraObject(builder);
            Wire.ParentScene.StartParentScene(builder);
            Wire.ParentScene.AddKind(builder, Wire.ParentSceneKind.Persistent);
            Offset<Wire.ParentScene> parent = Wire.ParentScene.EndParentScene(builder);
            VectorOffset events = Wire.GameObject.CreatePointerEventsVector(
                builder,
                Array.Empty<Wire.PointerEventKind>()
            );
            Wire.GameObject.StartGameObject(builder);
            Wire.GameObject.AddContent(builder, content.Value);
            Wire.GameObject.AddContentType(builder, Wire.GameObjectContent.CameraObject);
            Wire.GameObject.AddKind(builder, Wire.GameObjectKind.Camera);
            Wire.GameObject.AddPointerEvents(builder, events);
            Wire.GameObject.AddLocalTransform(
                builder,
                Wire.LocalTransform.CreateLocalTransform(builder, 0, 0, -10, 0, 0, 0, 1, 1, 1, 1)
            );
            Wire.GameObject.AddParentScene(builder, parent);
            Wire.GameObject.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, cameraId)
            );
            return Wire.GameObject.EndGameObject(builder);
        }

        private static Offset<Wire.GameObject> Target(
            FlatBufferBuilder builder,
            Guid targetId,
            Guid sceneId
        )
        {
            VectorOffset materials = Wire.PrimitiveObject.CreateMaterialsVector(
                builder,
                Array.Empty<Offset<Wire.MaterialAssignment>>()
            );
            Offset<Wire.PrimitiveObject> content = Wire.PrimitiveObject.CreatePrimitiveObject(
                builder,
                materials
            );
            Wire.ParentScene.StartParentScene(builder);
            Wire.ParentScene.AddSceneId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, sceneId)
            );
            Wire.ParentScene.AddKind(builder, Wire.ParentSceneKind.Scene);
            Offset<Wire.ParentScene> parent = Wire.ParentScene.EndParentScene(builder);
            VectorOffset events = Wire.GameObject.CreatePointerEventsVector(
                builder,
                new[]
                {
                    Wire.PointerEventKind.Down,
                    Wire.PointerEventKind.Up,
                    Wire.PointerEventKind.Click,
                }
            );
            Wire.GameObject.StartGameObject(builder);
            Wire.GameObject.AddContent(builder, content.Value);
            Wire.GameObject.AddContentType(builder, Wire.GameObjectContent.PrimitiveObject);
            Wire.GameObject.AddKind(builder, Wire.GameObjectKind.Cube);
            Wire.GameObject.AddPointerEvents(builder, events);
            Wire.GameObject.AddLocalTransform(
                builder,
                Wire.LocalTransform.CreateLocalTransform(builder, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1)
            );
            Wire.GameObject.AddParentScene(builder, parent);
            Wire.GameObject.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, targetId)
            );
            return Wire.GameObject.EndGameObject(builder);
        }

        private static ReadOnlyMemory<byte> Finish(
            FlatBufferBuilder builder,
            Guid sessionId,
            Offset<Wire.ResponseMessageEntry> entry
        )
        {
            VectorOffset messages = Wire.Response.CreateMessagesVector(builder, new[] { entry });
            Wire.Response.StartResponse(builder);
            Wire.Response.AddMessages(builder, messages);
            Wire.Response.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, sessionId)
            );
            Offset<Wire.Response> response = Wire.Response.EndResponse(builder);
            Wire.Response.FinishSizePrefixedResponseBuffer(builder, response);
            return builder.SizedByteArray();
        }
    }
}
