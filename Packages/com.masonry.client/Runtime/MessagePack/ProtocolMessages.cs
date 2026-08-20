#nullable enable

using System;
using System.Buffers;
using System.Collections.Generic;
using MessagePack;
using MessagePack.Formatters;

namespace Masonry
{
    internal static partial class ProtocolFormat
    {
        private static readonly string[] CoreErrorCodeVariants =
        {
            "InvalidEncoding",
            "LimitExceeded",
            "WrongSession",
            "DuplicateId",
            "UnknownCommand",
            "UnknownObject",
            "UnknownScene",
            "UnknownAsset",
            "AssetNotPrepared",
            "AssetTypeMismatch",
            "AssetInUse",
            "ComponentMissing",
            "InvalidComponentCount",
            "InvalidHierarchy",
            "InvalidProperty",
            "PropertyControlledByBillboard",
            "InfiniteWait",
            "EarlierBatchFailed",
            "HandlerNotRegistered",
            "HandlerFailed",
            "UnityException",
        };

        internal static void WriteConnect(ref MessagePackWriter writer, Connect value)
        {
            WriteArrayHeader(ref writer, 6);
            WriteString(ref writer, value.Platform);
            WriteString(ref writer, value.UnityVersion);
            WriteScreenSize(ref writer, value.Screen);
            WriteStrings(ref writer, value.CustomCommandTypes);
            WriteOptionalString(ref writer, value.PersistentDataPath);
            WriteOptionalString(ref writer, value.StreamingAssetsPath);
        }

        internal static void WriteBatchFailureClientMessage(
            ref MessagePackWriter writer,
            BatchFailed<CoreErrorCode> value
        )
        {
            WriteVariantHeader(ref writer, "BatchFailed");
            WriteArrayHeader(ref writer, 5);
            WriteSessionId(ref writer, value.SessionId);
            WriteBatchId(ref writer, value.BatchId);
            WriteOptionalCommandId(ref writer, value.CommandId);
            WriteCoreErrorCode(ref writer, value.ErrorCode);
            WriteString(ref writer, value.Message);
        }

        internal static void WriteActionClientMessage(ref MessagePackWriter writer, Action value)
        {
            WriteVariantHeader(ref writer, "Action");
            WriteAction(ref writer, value);
        }

        internal static void WriteCustomActionClientMessage<TPayload>(
            ref MessagePackWriter writer,
            CustomAction<TPayload> value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteVariantHeader(ref writer, "CustomAction");
            WriteCustomAction(ref writer, value, payloadFormatter, options);
        }

        internal static void WriteBatchFailureClientMessage<TError>(
            ref MessagePackWriter writer,
            BatchFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteVariantHeader(ref writer, "BatchFailed");
            WriteBatchFailed(ref writer, value, errorFormatter, options);
        }

        internal static void WriteOperationFailureClientMessage<TError>(
            ref MessagePackWriter writer,
            OperationFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteVariantHeader(ref writer, "OperationFailed");
            WriteOperationFailed(ref writer, value, errorFormatter, options);
        }

        internal static void WriteOperationFailureClientMessage(
            ref MessagePackWriter writer,
            OperationFailed<CoreErrorCode> value
        )
        {
            WriteVariantHeader(ref writer, "OperationFailed");
            WriteArrayHeader(ref writer, 5);
            WriteSessionId(ref writer, value.SessionId);
            WriteBatchId(ref writer, value.BatchId);
            WriteCommandId(ref writer, value.CommandId);
            WriteCoreErrorCode(ref writer, value.ErrorCode);
            WriteString(ref writer, value.Message);
        }

        internal static Connect ReadConnect(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 6);
            return new Connect(
                ReadString(ref reader),
                ReadString(ref reader),
                ReadScreenSize(ref reader),
                ReadStrings(ref reader),
                ReadOptionalString(ref reader),
                ReadOptionalString(ref reader)
            );
        }

        internal static void WriteResponse(ref MessagePackWriter writer, Response value)
        {
            WriteArrayHeader(ref writer, 2);
            WriteSessionId(ref writer, value.SessionId);
            writer.WriteArrayHeader(value.Messages.Count);
            foreach (ResponseMessage<Command> message in value.Messages)
            {
                WriteResponseMessage(ref writer, message);
            }
        }

        internal static Response ReadResponse(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 2);
            SessionId sessionId = ReadSessionId(ref reader);
            int count = reader.ReadArrayHeader();
            var messages = new ResponseMessage<Command>[count];
            for (int index = 0; index < count; index++)
            {
                messages[index] = ReadResponseMessage(ref reader);
            }

            return new Response(sessionId, messages);
        }

        internal static void WriteResponse<TPayload>(
            ref MessagePackWriter writer,
            Response<ICommand> value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteArrayHeader(ref writer, 2);
            WriteSessionId(ref writer, value.SessionId);
            writer.WriteArrayHeader(value.Messages.Count);
            foreach (ResponseMessage<ICommand> message in value.Messages)
            {
                WriteResponseMessage(ref writer, message, payloadFormatter, options);
            }
        }

        internal static Response<ICommand> ReadResponse<TPayload>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            ReadArrayHeader(ref reader, 2);
            SessionId sessionId = ReadSessionId(ref reader);
            int count = reader.ReadArrayHeader();
            var messages = new ResponseMessage<ICommand>[count];
            for (int index = 0; index < count; index++)
            {
                messages[index] = ReadResponseMessage(ref reader, payloadFormatter, options);
            }

            return new Response<ICommand>(sessionId, messages);
        }

        internal static Response<ICommand> ReadResponse(
            ref MessagePackReader reader,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        )
        {
            ReadArrayHeader(ref reader, 2);
            SessionId sessionId = ReadSessionId(ref reader);
            int count = reader.ReadArrayHeader();
            var messages = new ResponseMessage<ICommand>[count];
            for (int index = 0; index < count; index++)
            {
                messages[index] = ReadResponseMessage(ref reader, decodeCustomCommand);
            }

            return new Response<ICommand>(sessionId, messages);
        }

        internal static void WriteClientMessage<TError, TCustomActionPayload>(
            ref MessagePackWriter writer,
            ClientMessage<TError, TCustomActionPayload> value,
            IMessagePackFormatter<TError> errorFormatter,
            IMessagePackFormatter<TCustomActionPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            switch (value)
            {
                case ClientMessage<TError, TCustomActionPayload>.ActionMessage action:
                    WriteVariantHeader(ref writer, "Action");
                    WriteAction(ref writer, action.Action);
                    break;
                case ClientMessage<TError, TCustomActionPayload>.CustomActionMessage custom:
                    WriteVariantHeader(ref writer, "CustomAction");
                    WriteCustomAction(ref writer, custom.Action, payloadFormatter, options);
                    break;
                case ClientMessage<TError, TCustomActionPayload>.BatchFailedMessage failed:
                    WriteVariantHeader(ref writer, "BatchFailed");
                    WriteBatchFailed(ref writer, failed.Failure, errorFormatter, options);
                    break;
                case ClientMessage<TError, TCustomActionPayload>.OperationFailedMessage failed:
                    WriteVariantHeader(ref writer, "OperationFailed");
                    WriteOperationFailed(ref writer, failed.Failure, errorFormatter, options);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown client message value.");
            }
        }

        internal static ClientMessage<TError, TCustomActionPayload> ReadClientMessage<
            TError,
            TCustomActionPayload
        >(
            ref MessagePackReader reader,
            IMessagePackFormatter<TError> errorFormatter,
            IMessagePackFormatter<TCustomActionPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "Action" => new ClientMessage<TError, TCustomActionPayload>.ActionMessage(
                    ReadAction(ref reader)
                ),
                "CustomAction" => new ClientMessage<
                    TError,
                    TCustomActionPayload
                >.CustomActionMessage(ReadCustomAction(ref reader, payloadFormatter, options)),
                "BatchFailed" => new ClientMessage<TError, TCustomActionPayload>.BatchFailedMessage(
                    ReadBatchFailed(ref reader, errorFormatter, options)
                ),
                "OperationFailed" => new ClientMessage<
                    TError,
                    TCustomActionPayload
                >.OperationFailedMessage(ReadOperationFailed(ref reader, errorFormatter, options)),
                _ => throw UnknownVariant("client message", variant),
            };
        }

        private static void WriteResponseMessage(
            ref MessagePackWriter writer,
            ResponseMessage<Command> value
        )
        {
            switch (value)
            {
                case ResponseMessage<Command>.SnapshotMessage snapshot:
                    WriteVariantHeader(ref writer, "Snapshot");
                    WriteSnapshot(ref writer, snapshot.Snapshot);
                    break;
                case ResponseMessage<Command>.BatchMessage batch:
                    WriteVariantHeader(ref writer, "Batch");
                    WriteBatch(ref writer, batch.Batch);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown response message.");
            }
        }

        private static ResponseMessage<Command> ReadResponseMessage(ref MessagePackReader reader)
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "Snapshot" => new ResponseMessage<Command>.SnapshotMessage(
                    ReadSnapshot(ref reader)
                ),
                "Batch" => new ResponseMessage<Command>.BatchMessage(ReadBatch(ref reader)),
                _ => throw UnknownVariant("response message", variant),
            };
        }

        private static void WriteResponseMessage<TPayload>(
            ref MessagePackWriter writer,
            ResponseMessage<ICommand> value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            switch (value)
            {
                case ResponseMessage<ICommand>.SnapshotMessage snapshot:
                    WriteVariantHeader(ref writer, "Snapshot");
                    WriteSnapshot(ref writer, snapshot.Snapshot);
                    break;
                case ResponseMessage<ICommand>.BatchMessage batch:
                    WriteVariantHeader(ref writer, "Batch");
                    WriteBatch(ref writer, batch.Batch, payloadFormatter, options);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown response message.");
            }
        }

        private static ResponseMessage<ICommand> ReadResponseMessage<TPayload>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "Snapshot" => new ResponseMessage<ICommand>.SnapshotMessage(
                    ReadSnapshot(ref reader)
                ),
                "Batch" => new ResponseMessage<ICommand>.BatchMessage(
                    ReadBatch(ref reader, payloadFormatter, options)
                ),
                _ => throw UnknownVariant("response message", variant),
            };
        }

        private static ResponseMessage<ICommand> ReadResponseMessage(
            ref MessagePackReader reader,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        )
        {
            string variant = ReadVariantHeader(ref reader);
            return variant switch
            {
                "Snapshot" => new ResponseMessage<ICommand>.SnapshotMessage(
                    ReadSnapshot(ref reader)
                ),
                "Batch" => new ResponseMessage<ICommand>.BatchMessage(
                    ReadBatch(ref reader, decodeCustomCommand)
                ),
                _ => throw UnknownVariant("response message", variant),
            };
        }

        private static void WriteSnapshot(ref MessagePackWriter writer, Snapshot value)
        {
            WriteArrayHeader(ref writer, 8);
            WriteSessionId(ref writer, value.SessionId);
            WritePreparedAssets(ref writer, value.PreparedAssets);
            WriteScenes(ref writer, value.Scenes);
            WriteOptionalSceneId(ref writer, value.PrimarySceneId);
            WriteGameObjects(ref writer, value.Objects);
            WriteObjectId(ref writer, value.InputCameraId);
            writer.Write(value.IsInputDisabled);
            WriteKeyCodes(ref writer, value.GlobalKeys);
        }

        private static Snapshot ReadSnapshot(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 8);
            SessionId sessionId = ReadSessionId(ref reader);
            IReadOnlyList<PreparedAsset> assets = ReadPreparedAssets(ref reader);
            IReadOnlyList<MasonryScene> scenes = ReadScenes(ref reader);
            SceneId? primarySceneId = ReadOptionalSceneId(ref reader);
            IReadOnlyList<MasonryGameObject> objects = ReadGameObjects(ref reader);
            ObjectId inputCameraId = ReadObjectId(ref reader);
            bool inputDisabled = reader.ReadBoolean();
            IReadOnlyList<KeyCode> keys = ReadKeyCodes(ref reader);
            return new Snapshot(
                sessionId,
                assets,
                scenes,
                objects,
                inputCameraId,
                primarySceneId,
                inputDisabled,
                keys
            );
        }

        private static void WriteBatch(ref MessagePackWriter writer, Batch<Command> value)
        {
            WriteArrayHeader(ref writer, 5);
            WriteBatchId(ref writer, value.Id);
            WriteSessionId(ref writer, value.SessionId);
            WriteOptionalActionId(ref writer, value.CausedByActionId);
            WriteBatchStart(ref writer, value.Start);
            writer.WriteArrayHeader(value.Groups.Count);
            foreach (ParallelCommandGroup<Command> group in value.Groups)
            {
                WriteArrayHeader(ref writer, 1);
                writer.WriteArrayHeader(group.Commands.Count);
                foreach (Command command in group.Commands)
                {
                    WriteCommand(ref writer, command);
                }
            }
        }

        private static Batch ReadBatch(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 5);
            BatchId id = ReadBatchId(ref reader);
            SessionId sessionId = ReadSessionId(ref reader);
            ActionId? causedBy = ReadOptionalActionId(ref reader);
            BatchStart start = ReadBatchStart(ref reader);
            int groupCount = reader.ReadArrayHeader();
            var groups = new ParallelCommandGroup<Command>[groupCount];
            for (int groupIndex = 0; groupIndex < groupCount; groupIndex++)
            {
                ReadArrayHeader(ref reader, 1);
                int commandCount = reader.ReadArrayHeader();
                var commands = new Command[commandCount];
                for (int commandIndex = 0; commandIndex < commandCount; commandIndex++)
                {
                    commands[commandIndex] = ReadCommand(ref reader);
                }

                groups[groupIndex] = new ParallelCommandGroup<Command>(commands);
            }

            return new Batch(id, sessionId, groups, causedBy, start);
        }

        private static void WriteBatch<TPayload>(
            ref MessagePackWriter writer,
            Batch<ICommand> value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteArrayHeader(ref writer, 5);
            WriteBatchId(ref writer, value.Id);
            WriteSessionId(ref writer, value.SessionId);
            WriteOptionalActionId(ref writer, value.CausedByActionId);
            WriteBatchStart(ref writer, value.Start);
            writer.WriteArrayHeader(value.Groups.Count);
            foreach (ParallelCommandGroup<ICommand> group in value.Groups)
            {
                WriteArrayHeader(ref writer, 1);
                writer.WriteArrayHeader(group.Commands.Count);
                foreach (ICommand command in group.Commands)
                {
                    WriteAnyCommand(ref writer, command, payloadFormatter, options);
                }
            }
        }

        private static Batch<ICommand> ReadBatch<TPayload>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            ReadArrayHeader(ref reader, 5);
            BatchId id = ReadBatchId(ref reader);
            SessionId sessionId = ReadSessionId(ref reader);
            ActionId? causedBy = ReadOptionalActionId(ref reader);
            BatchStart start = ReadBatchStart(ref reader);
            int groupCount = reader.ReadArrayHeader();
            var groups = new ParallelCommandGroup<ICommand>[groupCount];
            for (int groupIndex = 0; groupIndex < groupCount; groupIndex++)
            {
                ReadArrayHeader(ref reader, 1);
                int commandCount = reader.ReadArrayHeader();
                var commands = new ICommand[commandCount];
                for (int commandIndex = 0; commandIndex < commandCount; commandIndex++)
                {
                    commands[commandIndex] = ReadAnyCommand(ref reader, payloadFormatter, options);
                }

                groups[groupIndex] = new ParallelCommandGroup<ICommand>(commands);
            }

            return new Batch<ICommand>(id, sessionId, groups, causedBy, start);
        }

        private static Batch<ICommand> ReadBatch(
            ref MessagePackReader reader,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        )
        {
            ReadArrayHeader(ref reader, 5);
            BatchId id = ReadBatchId(ref reader);
            SessionId sessionId = ReadSessionId(ref reader);
            ActionId? causedBy = ReadOptionalActionId(ref reader);
            BatchStart start = ReadBatchStart(ref reader);
            int groupCount = reader.ReadArrayHeader();
            var groups = new ParallelCommandGroup<ICommand>[groupCount];
            for (int groupIndex = 0; groupIndex < groupCount; groupIndex++)
            {
                ReadArrayHeader(ref reader, 1);
                int commandCount = reader.ReadArrayHeader();
                var commands = new ICommand[commandCount];
                for (int commandIndex = 0; commandIndex < commandCount; commandIndex++)
                {
                    commands[commandIndex] = ReadRegisteredCommand(ref reader, decodeCustomCommand);
                }

                groups[groupIndex] = new ParallelCommandGroup<ICommand>(commands);
            }

            return new Batch<ICommand>(id, sessionId, groups, causedBy, start);
        }

        private static ICommand ReadRegisteredCommand(
            ref MessagePackReader reader,
            Func<CommandId, string, bool, ReadOnlyMemory<byte>, ICommand> decodeCustomCommand
        )
        {
            string variant = ReadVariantHeader(ref reader);
            if (variant == "Core")
            {
                return ReadCommand(ref reader);
            }

            if (variant != "Custom")
            {
                throw UnknownVariant("command", variant);
            }

            ReadArrayHeader(ref reader, 4);
            CommandId id = ReadCommandId(ref reader);
            string type = ReadString(ref reader);
            bool blocking = reader.ReadBoolean();
            ReadOnlySequence<byte> raw = reader.ReadRaw();
            return decodeCustomCommand(id, type, blocking, raw.ToArray());
        }

        private static void WriteAction(ref MessagePackWriter writer, Action value)
        {
            WriteArrayHeader(ref writer, 3);
            WriteActionId(ref writer, value.Id);
            WriteSessionId(ref writer, value.SessionId);
            WriteActionBody(ref writer, value.Body);
        }

        private static Action ReadAction(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 3);
            return new Action(
                ReadActionId(ref reader),
                ReadSessionId(ref reader),
                ReadActionBody(ref reader)
            );
        }

        private static void WriteActionBody(ref MessagePackWriter writer, ActionBody value)
        {
            switch (value)
            {
                case ActionBody.PointerEnter action:
                    WritePointerBody(
                        ref writer,
                        "PointerEnter",
                        action.ObjectId,
                        action.PointerId,
                        action.ScreenPosition,
                        action.WorldHit
                    );
                    break;
                case ActionBody.PointerExit action:
                    WritePointerBody(
                        ref writer,
                        "PointerExit",
                        action.ObjectId,
                        action.PointerId,
                        action.ScreenPosition,
                        action.WorldHit
                    );
                    break;
                case ActionBody.PointerDown action:
                    WritePointerButtonBody(
                        ref writer,
                        "PointerDown",
                        action.ObjectId,
                        action.PointerId,
                        action.ScreenPosition,
                        action.WorldHit,
                        action.Button
                    );
                    break;
                case ActionBody.PointerUp action:
                    WritePointerButtonBody(
                        ref writer,
                        "PointerUp",
                        action.ObjectId,
                        action.PointerId,
                        action.ScreenPosition,
                        action.WorldHit,
                        action.Button
                    );
                    break;
                case ActionBody.PointerClick action:
                    WritePointerButtonBody(
                        ref writer,
                        "PointerClick",
                        action.ObjectId,
                        action.PointerId,
                        action.ScreenPosition,
                        action.WorldHit,
                        action.Button
                    );
                    break;
                case ActionBody.KeyDown action:
                    WriteKeyBody(ref writer, "KeyDown", action.Key);
                    break;
                case ActionBody.KeyUp action:
                    WriteKeyBody(ref writer, "KeyUp", action.Key);
                    break;
                default:
                    throw new MessagePackSerializationException("Unknown action body value.");
            }
        }

        private static ActionBody ReadActionBody(ref MessagePackReader reader)
        {
            string variant = ReadVariantHeader(ref reader);
            switch (variant)
            {
                case "PointerEnter":
                {
                    PointerFields fields = ReadPointerFields(ref reader, button: false);
                    return new ActionBody.PointerEnter(
                        fields.ObjectId,
                        fields.ScreenPosition,
                        fields.WorldHit,
                        fields.PointerId
                    );
                }
                case "PointerExit":
                {
                    PointerFields fields = ReadPointerFields(ref reader, button: false);
                    return new ActionBody.PointerExit(
                        fields.ObjectId,
                        fields.ScreenPosition,
                        fields.WorldHit,
                        fields.PointerId
                    );
                }
                case "PointerDown":
                {
                    PointerFields fields = ReadPointerFields(ref reader, button: true);
                    return new ActionBody.PointerDown(
                        fields.ObjectId,
                        fields.ScreenPosition,
                        fields.WorldHit,
                        fields.PointerId,
                        fields.Button
                    );
                }
                case "PointerUp":
                {
                    PointerFields fields = ReadPointerFields(ref reader, button: true);
                    return new ActionBody.PointerUp(
                        fields.ObjectId,
                        fields.ScreenPosition,
                        fields.WorldHit,
                        fields.PointerId,
                        fields.Button
                    );
                }
                case "PointerClick":
                {
                    PointerFields fields = ReadPointerFields(ref reader, button: true);
                    return new ActionBody.PointerClick(
                        fields.ObjectId,
                        fields.ScreenPosition,
                        fields.WorldHit,
                        fields.PointerId,
                        fields.Button
                    );
                }
                case "KeyDown":
                    return new ActionBody.KeyDown(ReadKeyPayload(ref reader));
                case "KeyUp":
                    return new ActionBody.KeyUp(ReadKeyPayload(ref reader));
                default:
                    throw UnknownVariant(nameof(ActionBody), variant);
            }
        }

        private static void WritePointerBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            int pointerId,
            ScreenPosition screenPosition,
            Vector3 worldHit
        )
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, 4);
            WriteObjectId(ref writer, objectId);
            writer.Write(pointerId);
            WriteScreenPosition(ref writer, screenPosition);
            WriteVector3(ref writer, worldHit);
        }

        private static void WritePointerButtonBody(
            ref MessagePackWriter writer,
            string variant,
            ObjectId objectId,
            int pointerId,
            ScreenPosition screenPosition,
            Vector3 worldHit,
            PointerButton button
        )
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, 5);
            WriteObjectId(ref writer, objectId);
            writer.Write(pointerId);
            WriteScreenPosition(ref writer, screenPosition);
            WriteVector3(ref writer, worldHit);
            WritePointerButton(ref writer, button);
        }

        private static void WriteKeyBody(ref MessagePackWriter writer, string variant, KeyCode key)
        {
            WriteVariantHeader(ref writer, variant);
            WriteArrayHeader(ref writer, 1);
            WriteKeyCode(ref writer, key);
        }

        private static PointerFields ReadPointerFields(ref MessagePackReader reader, bool button)
        {
            ReadArrayHeader(ref reader, button ? 5 : 4);
            ObjectId objectId = ReadObjectId(ref reader);
            int pointerId = reader.ReadInt32();
            ScreenPosition position = ReadScreenPosition(ref reader);
            Vector3 worldHit = ReadVector3(ref reader);
            PointerButton pointerButton = button
                ? ReadPointerButton(ref reader)
                : PointerButton.Left;
            return new PointerFields(objectId, pointerId, position, worldHit, pointerButton);
        }

        private static KeyCode ReadKeyPayload(ref MessagePackReader reader)
        {
            ReadArrayHeader(ref reader, 1);
            return ReadKeyCode(ref reader);
        }

        private static void WriteCustomAction<TPayload>(
            ref MessagePackWriter writer,
            CustomAction<TPayload> value,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteArrayHeader(ref writer, 4);
            WriteActionId(ref writer, value.Id);
            WriteSessionId(ref writer, value.SessionId);
            WriteString(ref writer, value.Type);
            payloadFormatter.Serialize(ref writer, value.Payload, options);
        }

        private static CustomAction<TPayload> ReadCustomAction<TPayload>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TPayload> payloadFormatter,
            MessagePackSerializerOptions options
        )
        {
            ReadArrayHeader(ref reader, 4);
            return new CustomAction<TPayload>(
                ReadActionId(ref reader),
                ReadSessionId(ref reader),
                ReadString(ref reader),
                payloadFormatter.Deserialize(ref reader, options)
            );
        }

        private static void WriteBatchFailed<TError>(
            ref MessagePackWriter writer,
            BatchFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteArrayHeader(ref writer, 5);
            WriteSessionId(ref writer, value.SessionId);
            WriteBatchId(ref writer, value.BatchId);
            WriteOptionalCommandId(ref writer, value.CommandId);
            errorFormatter.Serialize(ref writer, value.ErrorCode, options);
            WriteString(ref writer, value.Message);
        }

        private static BatchFailed<TError> ReadBatchFailed<TError>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            ReadArrayHeader(ref reader, 5);
            SessionId sessionId = ReadSessionId(ref reader);
            BatchId batchId = ReadBatchId(ref reader);
            CommandId? commandId = ReadOptionalCommandId(ref reader);
            TError error = errorFormatter.Deserialize(ref reader, options);
            string message = ReadString(ref reader);
            return new BatchFailed<TError>(sessionId, batchId, error, message, commandId);
        }

        private static void WriteOperationFailed<TError>(
            ref MessagePackWriter writer,
            OperationFailed<TError> value,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            WriteArrayHeader(ref writer, 5);
            WriteSessionId(ref writer, value.SessionId);
            WriteBatchId(ref writer, value.BatchId);
            WriteCommandId(ref writer, value.CommandId);
            errorFormatter.Serialize(ref writer, value.ErrorCode, options);
            WriteString(ref writer, value.Message);
        }

        private static OperationFailed<TError> ReadOperationFailed<TError>(
            ref MessagePackReader reader,
            IMessagePackFormatter<TError> errorFormatter,
            MessagePackSerializerOptions options
        )
        {
            ReadArrayHeader(ref reader, 5);
            return new OperationFailed<TError>(
                ReadSessionId(ref reader),
                ReadBatchId(ref reader),
                ReadCommandId(ref reader),
                errorFormatter.Deserialize(ref reader, options),
                ReadString(ref reader)
            );
        }

        private static void WriteCoreErrorCode(ref MessagePackWriter writer, CoreErrorCode value) =>
            WriteEnum(ref writer, (int)value, CoreErrorCodeVariants, nameof(CoreErrorCode));

        private static CoreErrorCode ReadCoreErrorCode(ref MessagePackReader reader) =>
            (CoreErrorCode)ReadEnum(ref reader, CoreErrorCodeVariants, nameof(CoreErrorCode));

        private readonly struct PointerFields
        {
            internal PointerFields(
                ObjectId objectId,
                int pointerId,
                ScreenPosition screenPosition,
                Vector3 worldHit,
                PointerButton button
            ) =>
                (ObjectId, PointerId, ScreenPosition, WorldHit, Button) = (
                    objectId,
                    pointerId,
                    screenPosition,
                    worldHit,
                    button
                );

            internal ObjectId ObjectId { get; }

            internal int PointerId { get; }

            internal ScreenPosition ScreenPosition { get; }

            internal Vector3 WorldHit { get; }

            internal PointerButton Button { get; }
        }
    }
}
