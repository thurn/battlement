#nullable enable

using System;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>Constructs UI event actions directly in reusable FlatBuffers storage.</summary>
    internal sealed class BattlementUiEventRequestWriter
    {
        private FlatBufferBuilder builder;
        private BattlementUiEventBodyWriter bodies;

        internal BattlementUiEventRequestWriter()
        {
            builder = new FlatBufferBuilder(2048);
            bodies = new BattlementUiEventBodyWriter(builder);
        }

        internal int AllocationBytes => builder.DataBuffer.Length;

        internal void TrimOversized()
        {
            if (AllocationBytes <= 1024 * 1024)
                return;
            builder = new FlatBufferBuilder(2048);
            bodies = new BattlementUiEventBodyWriter(builder);
        }

        internal ReadOnlyMemory<byte> Write(UiEventAction action)
        {
            if (action.Event.DefaultPrevented && !action.Event.Cancelable)
                throw new InvalidDataException("A default-prevented UI event must be cancelable.");
            builder.Clear();
            BattlementUiEventBodyOffset body = bodies.Write(action.Event.Body);

            Wire.UiEvent.StartUiEvent(builder);
            Wire.UiEvent.AddCancelable(builder, action.Event.Cancelable);
            Wire.UiEvent.AddDefaultPrevented(builder, action.Event.DefaultPrevented);
            Wire.UiEvent.AddKind(builder, checked((byte)body.Kind));
            Wire.UiEvent.AddBodyType(builder, body.Type);
            Wire.UiEvent.AddBody(builder, body.Offset);
            Wire.UiEvent.AddTargetId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, action.Event.TargetId.Value)
            );
            Offset<Wire.UiEvent> uiEvent = Wire.UiEvent.EndUiEvent(builder);

            Wire.UiEventAction.StartUiEventAction(builder);
            Wire.UiEventAction.AddEvent(builder, uiEvent);
            Wire.UiEventAction.AddSessionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, action.SessionId.Value)
            );
            Wire.UiEventAction.AddActionId(
                builder,
                BattlementFlatBufferWriter.WriteUuid(builder, action.Id.Value)
            );
            Offset<Wire.UiEventAction> root = Wire.UiEventAction.EndUiEventAction(builder);
            Wire.UiEventAction.FinishSizePrefixedUiEventActionBuffer(builder, root);
            if (builder.Offset > BattlementProtocolLimits.MaximumMessageBytes)
                throw new InvalidDataException("A UI event cannot exceed 16 MiB.");
            return builder.DataBuffer.ToReadOnlyMemory(builder.DataBuffer.Position, builder.Offset);
        }
    }

    internal readonly struct BattlementUiEventBodyOffset
    {
        internal BattlementUiEventBodyOffset(UiEventKind kind, Wire.UiEventBody type, int offset) =>
            (Kind, Type, Offset) = (kind, type, offset);

        internal UiEventKind Kind { get; }
        internal Wire.UiEventBody Type { get; }
        internal int Offset { get; }
    }
}
