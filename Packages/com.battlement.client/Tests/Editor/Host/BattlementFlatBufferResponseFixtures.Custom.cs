#nullable enable

using System;
using Battlement.CustomFixtures;
using Google.FlatBuffers;
using CoreWire = Battlement.FlatBuffers.Generated;
using FixtureWire = Battlement.FlatBuffers.FixtureGenerated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        internal static ReadOnlyMemory<byte> Write(Response<ICommand> response)
        {
            var builder = new FlatBufferBuilder(1024);
            var messages = new int[response.Messages.Count];
            for (int index = 0; index < messages.Length; index++)
            {
                if (response.Messages[index] is not ResponseMessage<ICommand>.BatchMessage batch)
                    throw new ArgumentException(
                        "The custom-command fixture only writes batch messages.",
                        nameof(response)
                    );
                messages[index] = FixtureWire
                    .ResponseMessageEntry.CreateResponseMessageEntry(
                        builder,
                        FixtureWire.ResponseMessage.Batch,
                        WriteCustomBatch(builder, batch.Batch).Value
                    )
                    .Value;
            }

            VectorOffset messageVector = WriteOffsetVector(builder, messages);
            FixtureWire.Response.StartResponse(builder);
            FixtureWire.Response.AddMessages(builder, messageVector);
            FixtureWire.Response.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, response.SessionId.Value)
            );
            Offset<FixtureWire.Response> root = FixtureWire.Response.EndResponse(builder);
            FixtureWire.Response.FinishSizePrefixedResponseBuffer(builder, root);
            return builder.SizedByteArray();
        }

        private static Offset<FixtureWire.Batch> WriteCustomBatch(
            FlatBufferBuilder builder,
            Batch<ICommand> batch
        )
        {
            var groups = new int[batch.Groups.Count];
            for (int groupIndex = 0; groupIndex < groups.Length; groupIndex++)
            {
                var commands = new int[batch.Groups[groupIndex].Commands.Count];
                for (int commandIndex = 0; commandIndex < commands.Length; commandIndex++)
                {
                    ICommand command = batch.Groups[groupIndex].Commands[commandIndex];
                    FixtureWire.FixtureCommand type;
                    int value;
                    switch (command)
                    {
                        case Command core:
                            type = FixtureWire
                                .FixtureCommand
                                .Battlement_FlatBuffers_Generated_CoreCommand;
                            value = WriteCommand(builder, core).Value;
                            break;
                        case CustomCommand<FlashPayload> custom:
                            type = FixtureWire.FixtureCommand.FlashCommand;
                            value = WriteFlashCommand(builder, custom).Value;
                            break;
                        default:
                            throw new ArgumentException(
                                "Unknown custom-command fixture value.",
                                nameof(batch)
                            );
                    }
                    commands[commandIndex] = FixtureWire
                        .CommandEntry.CreateCommandEntry(builder, type, value)
                        .Value;
                }
                VectorOffset commandVector = WriteOffsetVector(builder, commands);
                groups[groupIndex] = FixtureWire
                    .ParallelCommandGroup.CreateParallelCommandGroup(builder, commandVector)
                    .Value;
            }

            VectorOffset groupVector = WriteOffsetVector(builder, groups);
            Offset<CoreWire.PresentationControl> presentationControl = default;
            if (batch.PresentationControl is PresentationControl control)
            {
                CoreWire.PresentationControl.StartPresentationControl(builder);
                CoreWire.PresentationControl.AddPaused(builder, control.Paused);
                CoreWire.PresentationControl.AddOwnerId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, control.OwnerId.Value)
                );
                CoreWire.PresentationControl.AddWorkScope(builder, control.WorkScope);
                presentationControl = CoreWire.PresentationControl.EndPresentationControl(builder);
            }
            FixtureWire.Batch.StartBatch(builder);
            FixtureWire.Batch.AddGroups(builder, groupVector);
            FixtureWire.Batch.AddStart(builder, (CoreWire.BatchStart)batch.Start);
            if (batch.WorkScope is ulong workScope)
                FixtureWire.Batch.AddWorkScope(builder, workScope);
            if (batch.CancelScope is ulong cancelScope)
                FixtureWire.Batch.AddCancelScope(builder, cancelScope);
            if (batch.PresentationControl is not null)
                FixtureWire.Batch.AddPresentationControl(builder, presentationControl);
            if (batch.CausedByActionId is ActionId actionId)
                FixtureWire.Batch.AddCausedByActionId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, actionId.Value)
                );
            FixtureWire.Batch.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, batch.SessionId.Value)
            );
            FixtureWire.Batch.AddBatchId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, batch.Id.Value)
            );
            return FixtureWire.Batch.EndBatch(builder);
        }

        private static Offset<FixtureWire.FlashCommand> WriteFlashCommand(
            FlatBufferBuilder builder,
            CustomCommand<FlashPayload> command
        )
        {
            FixtureWire.FlashPayload.StartFlashPayload(builder);
            FixtureWire.FlashPayload.AddScale(builder, command.Payload.Scale);
            FixtureWire.FlashPayload.AddObjectId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, command.Payload.ObjectId.Value)
            );
            Offset<FixtureWire.FlashPayload> payload = FixtureWire.FlashPayload.EndFlashPayload(
                builder
            );
            StringOffset type = builder.CreateString(command.Type);
            FixtureWire.FlashCommand.StartFlashCommand(builder);
            FixtureWire.FlashCommand.AddPayload(builder, payload);
            FixtureWire.FlashCommand.AddCommandType(builder, type);
            FixtureWire.FlashCommand.AddBlocking(builder, command.IsBlocking);
            FixtureWire.FlashCommand.AddCommandId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, command.Id.Value)
            );
            return FixtureWire.FlashCommand.EndFlashCommand(builder);
        }

        private static VectorOffset WriteOffsetVector(FlatBufferBuilder builder, int[] values)
        {
            builder.StartVector(4, values.Length, 4);
            for (int index = values.Length - 1; index >= 0; index--)
                builder.AddOffset(values[index]);
            return builder.EndVector();
        }
    }
}
