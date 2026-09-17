#nullable enable

using System;
using System.IO;
using Google.FlatBuffers;
using CoreWire = Battlement.FlatBuffers.Generated;
using Wire = Battlement.FlatBuffers.FixtureGenerated;

namespace Battlement.CustomFixtures
{
    /// <summary>Generated-schema adapter for the fixture's typed command union.</summary>
    public sealed class FixtureFlatBufferResponseSchema
        : IBattlementFlatBufferResponseViewSchema,
            IBattlementFlatBufferClientSchema
    {
        public const string ContractDigest =
            "2e40dbf2290abf7d1ffaf1782d73830195ef34a77160ad2cdde66ddea93acc76";

        private readonly FlatBufferBuilder clientBuilder = new(1024);
        private readonly Func<object, byte> errorEncoder;
        private readonly Func<object, (ObjectId ObjectId, float Scale)> payloadEncoder;

        public int OwnedCoreCommandMaterializations { get; private set; }

        public FixtureFlatBufferResponseSchema()
            : this(
                error =>
                    error is Battlement.Integration.IntegrationFixtureError value
                        ? (byte)value
                        : throw new InvalidDataException("The integration error code is unknown."),
                payload =>
                    payload is Battlement.Integration.IntegrationFixturePayload value
                        ? (value.ObjectId, value.Scale)
                        : throw new InvalidDataException("The integration payload type is unknown.")
            ) { }

        public FixtureFlatBufferResponseSchema(
            Func<object, byte> errorEncoder,
            Func<object, (ObjectId ObjectId, float Scale)> payloadEncoder
        ) =>
            (this.errorEncoder, this.payloadEncoder) = (
                errorEncoder ?? throw new ArgumentNullException(nameof(errorEncoder)),
                payloadEncoder ?? throw new ArgumentNullException(nameof(payloadEncoder))
            );

        public string WireContractDigest => ContractDigest;

        public void ValidateResponse(ByteBuffer bytes)
        {
            if (bytes.Length is < 12 or > 16 * 1024 * 1024)
                throw new InvalidDataException("The fixture response size is invalid.");
            if (bytes.GetUint(0) != bytes.Length - 4)
                throw new InvalidDataException("The fixture response size prefix is invalid.");
            var verifier = new Verifier(
                bytes,
                new Options(64, 1_000_000, 64UL * 1024 * 1024, true, true)
            );
            if (!verifier.VerifyBuffer("BTRS", true, Wire.ResponseVerify.Verify))
                throw new InvalidDataException("The fixture response failed verification.");
            bytes.Position = FlatBufferConstants.SizePrefixLength;
            Wire.Response response = Root(bytes);
            Guid responseSession = BattlementFlatBufferCore.ReadUuid(response.SessionId, "session");
            if (response.MessagesLength > 256)
                throw new InvalidDataException("The fixture response has too many messages.");
            for (int messageIndex = 0; messageIndex < response.MessagesLength; messageIndex++)
            {
                Wire.ResponseMessageEntry entry = Message(bytes, messageIndex);
                if (
                    entry.MessageType
                    == Wire.ResponseMessage.Battlement_FlatBuffers_Generated_Snapshot
                )
                {
                    BattlementFlatBufferCore.ValidateSnapshot(
                        entry.MessageAsBattlement_FlatBuffers_Generated_Snapshot(),
                        responseSession
                    );
                    continue;
                }
                Wire.Batch batch = Batch(bytes, messageIndex);
                _ = BattlementFlatBufferCore.ReadUuid(batch.BatchId, "batch");
                if (
                    BattlementFlatBufferCore.ReadUuid(batch.SessionId, "batch session")
                    != responseSession
                )
                    throw new InvalidDataException("A fixture batch has the wrong session.");
                if (batch.CausedByActionId.HasValue)
                    _ = BattlementFlatBufferCore.ReadUuid(batch.CausedByActionId, "causing action");
                if ((byte)batch.Start > (byte)CoreWire.BatchStart.AfterEarlierAssetPreparation)
                    throw new InvalidDataException("A fixture batch start value is unknown.");
                if (batch.WorkScope == 0 || batch.CancelScope == 0)
                    throw new InvalidDataException("A work scope must be nonzero.");
                if (
                    batch.CancelScope.HasValue
                    && (batch.WorkScope.HasValue || batch.Start != CoreWire.BatchStart.Now)
                )
                    throw new InvalidDataException(
                        "Cancellation must be independent unowned work."
                    );
                if (batch.GroupsLength == 0 && !batch.CancelScope.HasValue)
                    throw new InvalidDataException("A fixture batch has no command groups.");
                for (int groupIndex = 0; groupIndex < batch.GroupsLength; groupIndex++)
                {
                    Wire.ParallelCommandGroup group = Group(bytes, messageIndex, groupIndex);
                    if (group.CommandsLength == 0)
                        throw new InvalidDataException("A fixture command group is empty.");
                    for (int commandIndex = 0; commandIndex < group.CommandsLength; commandIndex++)
                    {
                        Wire.CommandEntry command = Entry(
                            bytes,
                            messageIndex,
                            groupIndex,
                            commandIndex
                        );
                        if (batch.CancelScope.HasValue)
                        {
                            if (
                                command.CommandType
                                != Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand
                            )
                                throw new InvalidDataException(
                                    "Cancellation cleanup must contain core destroys."
                                );
                            var cleanup =
                                command.CommandAsBattlement_FlatBuffers_Generated_CoreCommand();
                            if (
                                cleanup.Kind != CoreWire.CoreCommandKind.VisualElementDestroy
                                && cleanup.Kind != CoreWire.CoreCommandKind.ObjectDestroy
                            )
                                throw new InvalidDataException(
                                    "Cancellation cleanup may only destroy objects."
                                );
                        }
                        switch (command.CommandType)
                        {
                            case Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand:
                                BattlementFlatBufferCore.ValidateCommand(
                                    command.CommandAsBattlement_FlatBuffers_Generated_CoreCommand()
                                );
                                break;
                            case Wire.FixtureCommand.FlashCommand:
                                ValidateFlash(command.CommandAsFlashCommand());
                                break;
                            case Wire.FixtureCommand.NONE:
                                break;
                            default:
                                throw new InvalidDataException("Unknown fixture command kind.");
                        }
                    }
                }
            }
            bytes.Position = 0;
        }

        public SessionId ReadSessionId(ByteBuffer bytes) =>
            new(BattlementFlatBufferCore.ReadUuid(Root(bytes).SessionId, "session"));

        public int ReadMessageCount(ByteBuffer bytes) => Root(bytes).MessagesLength;

        public bool IsSnapshot(ByteBuffer bytes, int messageIndex) =>
            Message(bytes, messageIndex).MessageType
            == Wire.ResponseMessage.Battlement_FlatBuffers_Generated_Snapshot;

        public CoreWire.Snapshot ReadSnapshotTable(ByteBuffer bytes, int messageIndex)
        {
            Wire.ResponseMessageEntry entry = Message(bytes, messageIndex);
            if (entry.MessageType != Wire.ResponseMessage.Battlement_FlatBuffers_Generated_Snapshot)
                throw new InvalidDataException("The fixture response message is not a snapshot.");
            return entry.MessageAsBattlement_FlatBuffers_Generated_Snapshot();
        }

        public BatchId ReadBatchId(ByteBuffer bytes, int messageIndex) =>
            new(BattlementFlatBufferCore.ReadUuid(Batch(bytes, messageIndex).BatchId, "batch"));

        public SessionId ReadBatchSessionId(ByteBuffer bytes, int messageIndex) =>
            new(
                BattlementFlatBufferCore.ReadUuid(
                    Batch(bytes, messageIndex).SessionId,
                    "batch session"
                )
            );

        public ActionId? ReadCausedByActionId(ByteBuffer bytes, int messageIndex)
        {
            Wire.Batch value = Batch(bytes, messageIndex);
            return value.CausedByActionId.HasValue
                ? new ActionId(
                    BattlementFlatBufferCore.ReadUuid(value.CausedByActionId, "causing action")
                )
                : null;
        }

        public BatchStart ReadBatchStart(ByteBuffer bytes, int messageIndex) =>
            (BatchStart)(byte)Batch(bytes, messageIndex).Start;

        public ulong? ReadWorkScope(ByteBuffer bytes, int messageIndex) =>
            Batch(bytes, messageIndex).WorkScope;

        public ulong? ReadCancelScope(ByteBuffer bytes, int messageIndex) =>
            Batch(bytes, messageIndex).CancelScope;

        public int ReadGroupCount(ByteBuffer bytes, int messageIndex) =>
            Batch(bytes, messageIndex).GroupsLength;

        public int ReadCommandCount(ByteBuffer bytes, int messageIndex, int groupIndex) =>
            Group(bytes, messageIndex, groupIndex).CommandsLength;

        public CommandId ReadCommandId(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        ) =>
            Entry(bytes, messageIndex, groupIndex, commandIndex).CommandType switch
            {
                Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand => new CommandId(
                    BattlementFlatBufferCore.ReadUuid(
                        Entry(bytes, messageIndex, groupIndex, commandIndex)
                            .CommandAsBattlement_FlatBuffers_Generated_CoreCommand()
                            .CommandId,
                        "command"
                    )
                ),
                Wire.FixtureCommand.FlashCommand => new CommandId(
                    BattlementFlatBufferCore.ReadUuid(
                        Entry(bytes, messageIndex, groupIndex, commandIndex)
                            .CommandAsFlashCommand()
                            .CommandId,
                        "command"
                    )
                ),
                _ => throw new InvalidDataException("Unknown fixture command kind."),
            };

        public bool ReadCommandIsBlocking(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        )
        {
            Wire.CommandEntry entry = Entry(bytes, messageIndex, groupIndex, commandIndex);
            return entry.CommandType switch
            {
                Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand => entry
                    .CommandAsBattlement_FlatBuffers_Generated_CoreCommand()
                    .Blocking,
                Wire.FixtureCommand.FlashCommand => entry.CommandAsFlashCommand().Blocking,
                _ => throw new InvalidDataException("Unknown fixture command kind."),
            };
        }

        public bool IsCoreCommand(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        ) =>
            Entry(bytes, messageIndex, groupIndex, commandIndex).CommandType
            == Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand;

        public CoreWire.CoreCommand ReadCoreCommandTable(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        )
        {
            Wire.CommandEntry entry = Entry(bytes, messageIndex, groupIndex, commandIndex);
            if (
                entry.CommandType
                != Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand
            )
                throw new InvalidDataException("The fixture command is not a core command.");
            return entry.CommandAsBattlement_FlatBuffers_Generated_CoreCommand();
        }

        public IBattlementCommandOperation? LaunchCustomCommand(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex,
            CommandId commandId,
            bool isBlocking,
            IBattlementFlatBufferCustomCommandDispatcher dispatcher,
            TimeSpan now
        )
        {
            Wire.CommandEntry entry = Entry(bytes, messageIndex, groupIndex, commandIndex);
            if (entry.CommandType != Wire.FixtureCommand.FlashCommand)
                throw new InvalidDataException("The fixture command is not a custom command.");
            Wire.FlashCommand command = entry.CommandAsFlashCommand();
            return dispatcher.Launch(
                commandId,
                command.CommandType,
                isBlocking,
                command.Payload!.Value,
                now
            );
        }

        public bool IsAssetPreparation(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        )
        {
            Wire.CommandEntry entry = Entry(bytes, messageIndex, groupIndex, commandIndex);
            return entry.CommandType
                    == Wire.FixtureCommand.Battlement_FlatBuffers_Generated_CoreCommand
                && entry.CommandAsBattlement_FlatBuffers_Generated_CoreCommand().Kind
                    == CoreWire.CoreCommandKind.AssetsReplaceSet;
        }

        public ReadOnlyMemory<byte> SerializeCustomAction<TPayload>(CustomAction<TPayload> value)
        {
            (ObjectId ObjectId, float Scale) payload = payloadEncoder(value.Payload!);
            clientBuilder.Clear();
            StringOffset type = clientBuilder.CreateString(value.Type);
            Offset<Wire.FlashPayload> payloadOffset = WritePayload(payload);
            Wire.FixtureAction.StartFixtureAction(clientBuilder);
            Wire.FixtureAction.AddPayload(clientBuilder, payloadOffset);
            Wire.FixtureAction.AddActionType(clientBuilder, type);
            Wire.FixtureAction.AddSessionId(clientBuilder, WriteUuid(value.SessionId.Value));
            Wire.FixtureAction.AddActionId(clientBuilder, WriteUuid(value.Id.Value));
            return Finish(
                Wire.FixtureClientBody.FixtureAction,
                Wire.FixtureAction.EndFixtureAction(clientBuilder).Value
            );
        }

        public ReadOnlyMemory<byte> SerializeBatchFailure<TError>(BatchFailed<TError> value)
        {
            Wire.FixtureError error = Error(value.ErrorCode);
            clientBuilder.Clear();
            StringOffset message = clientBuilder.CreateString(value.Message);
            Wire.FixtureBatchFailed.StartFixtureBatchFailed(clientBuilder);
            Wire.FixtureBatchFailed.AddMessage(clientBuilder, message);
            Wire.FixtureBatchFailed.AddError(clientBuilder, error);
            if (value.CommandId is CommandId commandId)
                Wire.FixtureBatchFailed.AddCommandId(clientBuilder, WriteUuid(commandId.Value));
            Wire.FixtureBatchFailed.AddBatchId(clientBuilder, WriteUuid(value.BatchId.Value));
            Wire.FixtureBatchFailed.AddSessionId(clientBuilder, WriteUuid(value.SessionId.Value));
            return Finish(
                Wire.FixtureClientBody.FixtureBatchFailed,
                Wire.FixtureBatchFailed.EndFixtureBatchFailed(clientBuilder).Value
            );
        }

        public ReadOnlyMemory<byte> SerializeOperationFailure<TError>(OperationFailed<TError> value)
        {
            Wire.FixtureError error = Error(value.ErrorCode);
            clientBuilder.Clear();
            StringOffset message = clientBuilder.CreateString(value.Message);
            Wire.FixtureOperationFailed.StartFixtureOperationFailed(clientBuilder);
            Wire.FixtureOperationFailed.AddMessage(clientBuilder, message);
            Wire.FixtureOperationFailed.AddError(clientBuilder, error);
            Wire.FixtureOperationFailed.AddCommandId(
                clientBuilder,
                WriteUuid(value.CommandId.Value)
            );
            Wire.FixtureOperationFailed.AddBatchId(clientBuilder, WriteUuid(value.BatchId.Value));
            Wire.FixtureOperationFailed.AddSessionId(
                clientBuilder,
                WriteUuid(value.SessionId.Value)
            );
            return Finish(
                Wire.FixtureClientBody.FixtureOperationFailed,
                Wire.FixtureOperationFailed.EndFixtureOperationFailed(clientBuilder).Value
            );
        }

        private static void ValidateFlash(Wire.FlashCommand value)
        {
            _ = BattlementFlatBufferCore.ReadUuid(value.CommandId, "command");
            if (value.CommandType != "fixture.character.flash")
                throw new InvalidDataException("Unknown fixture custom command type.");
            Wire.FlashPayload payload = value.Payload!.Value;
            _ = BattlementFlatBufferCore.ReadUuid(payload.ObjectId, "flash object");
            if (!float.IsFinite(payload.Scale) || payload.Scale < 0)
                throw new InvalidDataException("The fixture flash scale is invalid.");
        }

        private static Wire.Response Root(ByteBuffer bytes) =>
            Wire.Response.GetRootAsResponse(bytes);

        private static Wire.ResponseMessageEntry Message(ByteBuffer bytes, int index)
        {
            Wire.Response response = Root(bytes);
            if ((uint)index >= (uint)response.MessagesLength)
                throw new ArgumentOutOfRangeException(nameof(index));
            return response.Messages(index)!.Value;
        }

        private static Wire.Batch Batch(ByteBuffer bytes, int messageIndex)
        {
            Wire.ResponseMessageEntry entry = Message(bytes, messageIndex);
            if (entry.MessageType != Wire.ResponseMessage.Batch)
                throw new InvalidDataException("The fixture response message is not a batch.");
            return entry.MessageAsBatch();
        }

        private static Wire.ParallelCommandGroup Group(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex
        )
        {
            Wire.Batch batch = Batch(bytes, messageIndex);
            if ((uint)groupIndex >= (uint)batch.GroupsLength)
                throw new ArgumentOutOfRangeException(nameof(groupIndex));
            return batch.Groups(groupIndex)!.Value;
        }

        private static Wire.CommandEntry Entry(
            ByteBuffer bytes,
            int messageIndex,
            int groupIndex,
            int commandIndex
        )
        {
            Wire.ParallelCommandGroup group = Group(bytes, messageIndex, groupIndex);
            if ((uint)commandIndex >= (uint)group.CommandsLength)
                throw new ArgumentOutOfRangeException(nameof(commandIndex));
            return group.Commands(commandIndex)!.Value;
        }

        private Offset<Wire.FlashPayload> WritePayload((ObjectId ObjectId, float Scale) value)
        {
            if (!float.IsFinite(value.Scale) || value.Scale < 0)
                throw new InvalidDataException("The fixture flash scale is invalid.");
            Wire.FlashPayload.StartFlashPayload(clientBuilder);
            Wire.FlashPayload.AddScale(clientBuilder, value.Scale);
            Wire.FlashPayload.AddObjectId(clientBuilder, WriteUuid(value.ObjectId.Value));
            return Wire.FlashPayload.EndFlashPayload(clientBuilder);
        }

        private Offset<CoreWire.Uuid> WriteUuid(Guid value)
        {
            if (value == Guid.Empty)
                throw new InvalidDataException("Protocol UUIDs must be nonzero.");
            Span<byte> mixed = stackalloc byte[16];
            value.TryWriteBytes(mixed);
            ReadOnlySpan<byte> order = stackalloc byte[16]
            {
                3,
                2,
                1,
                0,
                5,
                4,
                7,
                6,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
            };
            clientBuilder.Prep(1, 16);
            for (int index = 15; index >= 0; index--)
                clientBuilder.PutByte(mixed[order[index]]);
            return new Offset<CoreWire.Uuid>(clientBuilder.Offset);
        }

        private ReadOnlyMemory<byte> Finish(Wire.FixtureClientBody type, int body)
        {
            Offset<Wire.FixtureClientMessage> root =
                Wire.FixtureClientMessage.CreateFixtureClientMessage(clientBuilder, type, body);
            clientBuilder.FinishSizePrefixed(root.Value, "BTCM");
            return clientBuilder.DataBuffer.ToReadOnlyMemory(
                clientBuilder.DataBuffer.Position,
                clientBuilder.Offset
            );
        }

        private Wire.FixtureError Error<TError>(TError value)
        {
            byte encoded = errorEncoder(value!);
            if (encoded > (byte)Wire.FixtureError.Delayed)
                throw new InvalidDataException("The fixture error code is unknown.");
            return (Wire.FixtureError)encoded;
        }
    }
}
