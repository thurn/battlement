#nullable enable

using System;
using System.Collections.Generic;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    /// <summary>Encodes host-test responses through the production FlatBuffer schema.</summary>
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        internal static ReadOnlyMemory<byte> Write(Response response)
        {
            var builder = new FlatBufferBuilder(4096);
            var messages = new int[response.Messages.Count];
            for (int index = 0; index < messages.Length; index++)
            {
                ResponseMessage<Command> message = response.Messages[index];
                Wire.ResponseMessage type;
                int value;
                switch (message)
                {
                    case ResponseMessage<Command>.SnapshotMessage snapshot:
                        type = Wire.ResponseMessage.Snapshot;
                        value = WriteSnapshot(builder, snapshot.Snapshot).Value;
                        break;
                    case ResponseMessage<Command>.BatchMessage batch:
                        type = Wire.ResponseMessage.Batch;
                        value = WriteBatch(builder, batch.Batch).Value;
                        break;
                    default:
                        throw new ArgumentException("Unknown response message.", nameof(response));
                }
                messages[index] = Wire
                    .ResponseMessageEntry.CreateResponseMessageEntry(builder, type, value)
                    .Value;
            }
            VectorOffset messageVector = OffsetVector(builder, messages);
            Wire.Response.StartResponse(builder);
            Wire.Response.AddMessages(builder, messageVector);
            Wire.Response.AddSessionId(builder, Uuid(builder, response.SessionId.Value));
            Offset<Wire.Response> root = Wire.Response.EndResponse(builder);
            Wire.Response.FinishSizePrefixedResponseBuffer(builder, root);
            return builder.SizedByteArray();
        }

        private static Offset<Wire.Batch> WriteBatch(
            FlatBufferBuilder builder,
            Batch<Command> batch
        )
        {
            var groups = new int[batch.Groups.Count];
            for (int groupIndex = 0; groupIndex < groups.Length; groupIndex++)
            {
                IReadOnlyList<Command> values = batch.Groups[groupIndex].Commands;
                var commands = new int[values.Count];
                for (int commandIndex = 0; commandIndex < commands.Length; commandIndex++)
                {
                    Offset<Wire.CoreCommand> command = WriteCommand(builder, values[commandIndex]);
                    commands[commandIndex] = Wire
                        .CommandEntry.CreateCommandEntry(
                            builder,
                            Wire.CommandEntryPayload.CoreCommand,
                            command.Value
                        )
                        .Value;
                }
                VectorOffset commandVector = OffsetVector(builder, commands);
                groups[groupIndex] = Wire
                    .ParallelCommandGroup.CreateParallelCommandGroup(builder, commandVector)
                    .Value;
            }
            VectorOffset groupVector = OffsetVector(builder, groups);
            Wire.Batch.StartBatch(builder);
            Wire.Batch.AddGroups(builder, groupVector);
            Wire.Batch.AddStart(builder, (Wire.BatchStart)batch.Start);
            if (batch.WorkScope is ulong owner)
                Wire.Batch.AddWorkScope(builder, owner);
            if (batch.CancelScope is ulong canceled)
                Wire.Batch.AddCancelScope(builder, canceled);
            if (batch.CausedByActionId is ActionId actionId)
                Wire.Batch.AddCausedByActionId(builder, Uuid(builder, actionId.Value));
            Wire.Batch.AddSessionId(builder, Uuid(builder, batch.SessionId.Value));
            Wire.Batch.AddBatchId(builder, Uuid(builder, batch.Id.Value));
            return Wire.Batch.EndBatch(builder);
        }

        private static Offset<Wire.Snapshot> WriteSnapshot(
            FlatBufferBuilder builder,
            Snapshot snapshot
        )
        {
            var assets = new int[snapshot.PreparedAssets.Count];
            for (int index = 0; index < assets.Length; index++)
                assets[index] = WriteAsset(builder, snapshot.PreparedAssets[index]).Value;
            VectorOffset assetVector = OffsetVector(builder, assets);

            var scenes = new int[snapshot.Scenes.Count];
            for (int index = 0; index < scenes.Length; index++)
            {
                BattlementScene scene = snapshot.Scenes[index];
                StringOffset address = builder.CreateString(scene.Address.Value);
                Wire.Scene.StartScene(builder);
                Wire.Scene.AddAddress(builder, address);
                Wire.Scene.AddSceneId(builder, Uuid(builder, scene.Id.Value));
                scenes[index] = Wire.Scene.EndScene(builder).Value;
            }
            VectorOffset sceneVector = OffsetVector(builder, scenes);

            var objects = new int[snapshot.Objects.Count];
            for (int index = 0; index < objects.Length; index++)
                objects[index] = WriteObject(builder, snapshot.Objects[index]).Value;
            VectorOffset objectVector = OffsetVector(builder, objects);

            IReadOnlyList<UiDocument> documents = snapshot.Ui ?? Array.Empty<UiDocument>();
            var ui = new int[documents.Count];
            for (int index = 0; index < ui.Length; index++)
                ui[index] = WriteUiDocument(builder, documents[index]).Value;
            VectorOffset uiVector = OffsetVector(builder, ui);

            Offset<Wire.PanelInputConfiguration> panelInput = WritePanelInput(
                builder,
                snapshot.PanelInputConfiguration ?? new PanelInputConfigurationValue()
            );
            var keys = new short[snapshot.GlobalKeys.Count];
            for (int index = 0; index < keys.Length; index++)
                keys[index] = (short)snapshot.GlobalKeys[index];
            VectorOffset keyVector = EnumVector(builder, keys);
            Offset<Wire.ControllerInputSettings>? controller = snapshot.ControllerInput is null
                ? null
                : WriteControllerInput(builder, snapshot.ControllerInput);

            Wire.Snapshot.StartSnapshot(builder);
            if (controller.HasValue)
                Wire.Snapshot.AddControllerInput(builder, controller.Value);
            Wire.Snapshot.AddGlobalKeys(builder, keyVector);
            Wire.Snapshot.AddInputDisabled(builder, snapshot.IsInputDisabled);
            if (snapshot.InputCameraId is ObjectId cameraId)
                Wire.Snapshot.AddInputCameraId(builder, Uuid(builder, cameraId.Value));
            Wire.Snapshot.AddPanelInputConfiguration(builder, panelInput);
            Wire.Snapshot.AddUi(builder, uiVector);
            Wire.Snapshot.AddObjects(builder, objectVector);
            if (snapshot.PrimarySceneId is SceneId primarySceneId)
                Wire.Snapshot.AddPrimarySceneId(builder, Uuid(builder, primarySceneId.Value));
            Wire.Snapshot.AddScenes(builder, sceneVector);
            Wire.Snapshot.AddPreparedAssets(builder, assetVector);
            Wire.Snapshot.AddSessionId(builder, Uuid(builder, snapshot.SessionId.Value));
            return Wire.Snapshot.EndSnapshot(builder);
        }

        private static Offset<Wire.PreparedAsset> WriteAsset(
            FlatBufferBuilder builder,
            PreparedAsset asset
        )
        {
            (Wire.PreparedAssetKind kind, string address) = asset switch
            {
                PreparedAsset.Scene value => (Wire.PreparedAssetKind.Scene, value.Address.Value),
                PreparedAsset.Mesh value => (Wire.PreparedAssetKind.Mesh, value.Address.Value),
                PreparedAsset.Prefab value => (Wire.PreparedAssetKind.Prefab, value.Address.Value),
                PreparedAsset.ParticleEffect value => (
                    Wire.PreparedAssetKind.ParticleEffect,
                    value.Address.Value
                ),
                PreparedAsset.MaterialParameters value => (
                    Wire.PreparedAssetKind.MaterialParameters,
                    value.Address.Value
                ),
                PreparedAsset.Material value => (
                    Wire.PreparedAssetKind.Material,
                    value.Address.Value
                ),
                PreparedAsset.Texture value => (
                    Wire.PreparedAssetKind.Texture,
                    value.Address.Value
                ),
                PreparedAsset.Sprite value => (Wire.PreparedAssetKind.Sprite, value.Address.Value),
                PreparedAsset.VectorImage value => (
                    Wire.PreparedAssetKind.VectorImage,
                    value.Address.Value
                ),
                PreparedAsset.RenderTexture value => (
                    Wire.PreparedAssetKind.RenderTexture,
                    value.Address.Value
                ),
                PreparedAsset.AudioClip value => (
                    Wire.PreparedAssetKind.AudioClip,
                    value.Address.Value
                ),
                PreparedAsset.TextMeshProFont value => (
                    Wire.PreparedAssetKind.TextMeshProFont,
                    value.Address.Value
                ),
                PreparedAsset.UiFont value => (Wire.PreparedAssetKind.UiFont, value.Address.Value),
                _ => throw new ArgumentException("Unknown prepared asset.", nameof(asset)),
            };
            StringOffset assetAddress = builder.CreateString(address);
            VectorOffset parameters = default;
            if (asset is PreparedAsset.MaterialParameters material)
            {
                var entries = new int[material.Parameters.Count];
                for (int i = 0; i < entries.Length; i++)
                {
                    MaterialParameterDeclaration p = material.Parameters[i];
                    entries[i] = Wire
                        .MaterialParameterDeclaration.CreateMaterialParameterDeclaration(
                            builder,
                            builder.CreateString(p.Name),
                            (Wire.MaterialParameterKind)p.Kind
                        )
                        .Value;
                }
                parameters = OffsetVector(builder, entries);
            }
            return Wire.PreparedAsset.CreatePreparedAsset(builder, kind, assetAddress, parameters);
        }

        private static Offset<Wire.PanelInputConfiguration> WritePanelInput(
            FlatBufferBuilder builder,
            PanelInputConfigurationValue value
        )
        {
            (Wire.InteractionDistanceKind kind, double distance) =
                value.MaximumInteractionDistance switch
                {
                    null => (Wire.InteractionDistanceKind.Unbounded, 0),
                    InteractionDistance.Unbounded => (Wire.InteractionDistanceKind.Unbounded, 0),
                    InteractionDistance.Inclusive inclusive => (
                        Wire.InteractionDistanceKind.Inclusive,
                        inclusive.Value
                    ),
                    _ => throw new ArgumentException(
                        "Unknown interaction distance.",
                        nameof(value)
                    ),
                };
            return Wire.PanelInputConfiguration.CreatePanelInputConfiguration(
                builder,
                value.InteractionLayers.Value,
                kind,
                (float)distance,
                (Wire.PanelInputRedirection)value.InputRedirection
            );
        }

        private static Offset<Wire.ControllerInputSettings> WriteControllerInput(
            FlatBufferBuilder builder,
            ControllerInputSettings value
        )
        {
            var buttons = new byte[value.Buttons.Count];
            for (int index = 0; index < buttons.Length; index++)
                buttons[index] = (byte)value.Buttons[index];
            VectorOffset buttonVector = ByteVector(builder, buttons);
            return Wire.ControllerInputSettings.CreateControllerInputSettings(
                builder,
                buttonVector,
                value.NavigationEnabled,
                value.StickDeadZone,
                value.RepeatDelay is null ? null : Milliseconds(value.RepeatDelay.Value),
                value.RepeatInterval is null ? null : Milliseconds(value.RepeatInterval.Value)
            );
        }

        private static Offset<Wire.Uuid> Uuid(FlatBufferBuilder builder, Guid value) =>
            BattlementFlatBufferWriter.WriteUuid(builder, value);

        private static VectorOffset OffsetVector(FlatBufferBuilder builder, int[] values)
        {
            builder.StartVector(4, values.Length, 4);
            for (int index = values.Length - 1; index >= 0; index--)
                builder.AddOffset(values[index]);
            return builder.EndVector();
        }

        private static VectorOffset EnumVector(FlatBufferBuilder builder, short[] values)
        {
            builder.StartVector(2, values.Length, 2);
            for (int index = values.Length - 1; index >= 0; index--)
                builder.AddShort(values[index]);
            return builder.EndVector();
        }

        private static VectorOffset ByteVector(FlatBufferBuilder builder, byte[] values)
        {
            builder.StartVector(1, values.Length, 1);
            for (int index = values.Length - 1; index >= 0; index--)
                builder.AddByte(values[index]);
            return builder.EndVector();
        }
    }
}
