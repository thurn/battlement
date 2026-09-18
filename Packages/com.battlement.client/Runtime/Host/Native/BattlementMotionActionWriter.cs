#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Writes one normalized Motion event batch directly into client-message storage.
    /// </summary>
    internal sealed class BattlementMotionActionWriter
    {
        private const int MaximumRecords = 262_144;
        private readonly FlatBufferBuilder builder;
        private readonly BattlementMotionValueWriter values;
        private Offset<Wire.MotionLifecycleEvent>[] eventOffsets = Array.Empty<
            Offset<Wire.MotionLifecycleEvent>
        >();
        private Offset<Wire.MotionPresentationSample>[] sampleOffsets = Array.Empty<
            Offset<Wire.MotionPresentationSample>
        >();
        private Offset<Wire.MotionValueSample>[] valueSampleOffsets = Array.Empty<
            Offset<Wire.MotionValueSample>
        >();
        private Offset<Wire.MotionPlaybackEvent>[] playbackOffsets = Array.Empty<
            Offset<Wire.MotionPlaybackEvent>
        >();
        private Offset<Wire.MotionGestureEvent>[] gestureOffsets = Array.Empty<
            Offset<Wire.MotionGestureEvent>
        >();
        private Offset<Wire.MotionSequenceLabelEvent>[] labelOffsets = Array.Empty<
            Offset<Wire.MotionSequenceLabelEvent>
        >();

        internal BattlementMotionActionWriter(FlatBufferBuilder builder)
        {
            this.builder = builder;
            values = new BattlementMotionValueWriter(builder);
        }

        internal Offset<Wire.MotionAction> Write(MotionEventBatch batch)
        {
            IReadOnlyList<MotionValueSample> valueSamples =
                batch.ValueSamples ?? Array.Empty<MotionValueSample>();
            IReadOnlyList<MotionPlaybackEvent> playbackEvents =
                batch.PlaybackEvents ?? Array.Empty<MotionPlaybackEvent>();
            IReadOnlyList<MotionGestureEvent> gestureEvents =
                batch.GestureEvents ?? Array.Empty<MotionGestureEvent>();
            IReadOnlyList<MotionSequenceLabelEvent> labelEvents =
                batch.LabelEvents ?? Array.Empty<MotionSequenceLabelEvent>();
            ValidateSequence(batch);
            Limit(batch.Events.Count);
            Limit(batch.Samples.Count);
            Limit(valueSamples.Count);
            Limit(playbackEvents.Count);
            Limit(gestureEvents.Count);
            Limit(labelEvents.Count);

            VectorOffset events = WriteEvents(batch.Events);
            VectorOffset samples = WriteSamples(batch.Samples);
            VectorOffset sampledValues = WriteValueSamples(valueSamples);
            VectorOffset playbacks = WritePlaybackEvents(playbackEvents);
            VectorOffset gestures = WriteGestureEvents(gestureEvents);
            VectorOffset labels = WriteLabelEvents(labelEvents);
            Offset<Wire.MotionEventBatch> wireBatch = Wire.MotionEventBatch.CreateMotionEventBatch(
                builder,
                batch.FirstSequence,
                batch.LastSequence,
                events,
                samples,
                sampledValues,
                playbacks,
                gestures,
                labels
            );
            return Wire.MotionAction.CreateMotionAction(builder, wireBatch);
        }

        private VectorOffset WriteEvents(IReadOnlyList<MotionLifecycleEvent> records)
        {
            Ensure(ref eventOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionLifecycleEvent value = records[index];
                (Wire.MotionLifecycleKind kind, uint first, uint last) = value.Kind switch
                {
                    MotionEventKind.Activated => (Wire.MotionLifecycleKind.Activated, 0u, 0u),
                    MotionEventKind.Started => (Wire.MotionLifecycleKind.Started, 0u, 0u),
                    MotionEventKind.Repeated repeated => Repeated(repeated),
                    MotionEventKind.Completed => (Wire.MotionLifecycleKind.Completed, 0u, 0u),
                    MotionEventKind.Stopped => (Wire.MotionLifecycleKind.Stopped, 0u, 0u),
                    MotionEventKind.Cancelled => (Wire.MotionLifecycleKind.Cancelled, 0u, 0u),
                    _ => throw new InvalidDataException("Unknown Motion lifecycle kind."),
                };
                Wire.MotionLifecycleEvent.StartMotionLifecycleEvent(builder);
                Wire.MotionLifecycleEvent.AddRepeatLast(builder, last);
                Wire.MotionLifecycleEvent.AddRepeatFirst(builder, first);
                Wire.MotionLifecycleEvent.AddKind(builder, kind);
                Wire.MotionLifecycleEvent.AddElapsedMicros(builder, value.ElapsedMicros);
                Wire.MotionLifecycleEvent.AddGeneration(builder, value.Generation);
                Wire.MotionLifecycleEvent.AddSlot(builder, value.Slot);
                Wire.MotionLifecycleEvent.AddDescriptorId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.DescriptorId.Value)
                );
                Wire.MotionLifecycleEvent.AddSequence(builder, value.Sequence);
                eventOffsets[index] = Wire.MotionLifecycleEvent.EndMotionLifecycleEvent(builder);
            }
            Wire.MotionEventBatch.StartEventsVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(eventOffsets[index].Value);
            return builder.EndVector();
        }

        private VectorOffset WriteSamples(IReadOnlyList<MotionPresentationSample> records)
        {
            Ensure(ref sampleOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionPresentationSample value = records[index];
                VectorOffset properties = values.WritePropertyValues(value.Values);
                Wire.MotionPresentationSample.StartMotionPresentationSample(builder);
                Wire.MotionPresentationSample.AddValues(builder, properties);
                Wire.MotionPresentationSample.AddElapsedMicros(builder, value.ElapsedMicros);
                Wire.MotionPresentationSample.AddGeneration(builder, value.Generation);
                Wire.MotionPresentationSample.AddSlot(builder, value.Slot);
                Wire.MotionPresentationSample.AddDescriptorId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.DescriptorId.Value)
                );
                sampleOffsets[index] = Wire.MotionPresentationSample.EndMotionPresentationSample(
                    builder
                );
            }
            Wire.MotionEventBatch.StartSamplesVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(sampleOffsets[index].Value);
            return builder.EndVector();
        }

        private VectorOffset WriteValueSamples(IReadOnlyList<MotionValueSample> records)
        {
            Ensure(ref valueSampleOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionValueSample value = records[index];
                MotionValueOffset encodedValue = values.Write(value.Value);
                MotionValueOffset velocity = values.Write(value.Velocity);
                Wire.MotionValueSample.StartMotionValueSample(builder);
                Wire.MotionValueSample.AddDiscontinuity(builder, value.Discontinuity);
                Wire.MotionValueSample.AddVelocity(builder, velocity.Offset);
                Wire.MotionValueSample.AddVelocityType(builder, velocity.Type);
                Wire.MotionValueSample.AddValue(builder, encodedValue.Offset);
                Wire.MotionValueSample.AddValueType(builder, encodedValue.Type);
                Wire.MotionValueSample.AddFrame(builder, value.Frame);
                Wire.MotionValueSample.AddValueId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.ValueId.Value)
                );
                Wire.MotionValueSample.AddSubscriptionId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.SubscriptionId.Value)
                );
                valueSampleOffsets[index] = Wire.MotionValueSample.EndMotionValueSample(builder);
            }
            Wire.MotionEventBatch.StartValueSamplesVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(valueSampleOffsets[index].Value);
            return builder.EndVector();
        }

        private VectorOffset WritePlaybackEvents(IReadOnlyList<MotionPlaybackEvent> records)
        {
            Ensure(ref playbackOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionPlaybackEvent value = records[index];
                if ((uint)value.Outcome > (uint)MotionPlaybackOutcome.Failed)
                    throw new InvalidDataException("Unknown Motion playback outcome.");
                Wire.MotionPlaybackEvent.StartMotionPlaybackEvent(builder);
                Wire.MotionPlaybackEvent.AddOutcome(
                    builder,
                    (Wire.MotionPlaybackOutcome)value.Outcome
                );
                Wire.MotionPlaybackEvent.AddGeneration(builder, value.Generation);
                Wire.MotionPlaybackEvent.AddPlaybackId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.PlaybackId.Value)
                );
                playbackOffsets[index] = Wire.MotionPlaybackEvent.EndMotionPlaybackEvent(builder);
            }
            Wire.MotionEventBatch.StartPlaybackEventsVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(playbackOffsets[index].Value);
            return builder.EndVector();
        }

        private VectorOffset WriteGestureEvents(IReadOnlyList<MotionGestureEvent> records)
        {
            Ensure(ref gestureOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionGestureEvent value = records[index];
                if ((uint)value.Kind > (uint)MotionGestureEventKind.InViewLeave)
                    throw new InvalidDataException("Unknown Motion gesture kind.");
                if ((uint)value.Device > (uint)MotionPointerDevice.Gamepad)
                    throw new InvalidDataException("Unknown Motion pointer device.");
                if (
                    value.Axis is MotionGestureAxis axis
                    && (uint)axis > (uint)MotionGestureAxis.Both
                )
                    throw new InvalidDataException("Unknown Motion gesture axis.");
                Wire.MotionGestureEvent.StartMotionGestureEvent(builder);
                Wire.MotionGestureEvent.AddConstrained(builder, value.Constrained);
                Wire.MotionGestureEvent.AddMomentumGeneration(builder, value.MomentumGeneration);
                if (value.Axis is MotionGestureAxis selected)
                {
                    Wire.MotionGestureEvent.AddAxis(builder, (Wire.MotionGestureAxis)selected);
                    Wire.MotionGestureEvent.AddHasAxis(builder, true);
                }
                Wire.MotionGestureEvent.AddVelocity(builder, Vector(value.Velocity));
                Wire.MotionGestureEvent.AddOffset(builder, Vector(value.Offset));
                Wire.MotionGestureEvent.AddDelta(builder, Vector(value.Delta));
                Wire.MotionGestureEvent.AddPoint(builder, Vector(value.Point));
                Wire.MotionGestureEvent.AddDevice(builder, (Wire.MotionPointerDevice)value.Device);
                Wire.MotionGestureEvent.AddPointerId(builder, value.PointerId);
                Wire.MotionGestureEvent.AddKind(builder, (Wire.MotionGestureEventKind)value.Kind);
                Wire.MotionGestureEvent.AddGeneration(builder, value.Generation);
                Wire.MotionGestureEvent.AddDescriptorId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.DescriptorId.Value)
                );
                gestureOffsets[index] = Wire.MotionGestureEvent.EndMotionGestureEvent(builder);
            }
            Wire.MotionEventBatch.StartGestureEventsVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(gestureOffsets[index].Value);
            return builder.EndVector();
        }

        private VectorOffset WriteLabelEvents(IReadOnlyList<MotionSequenceLabelEvent> records)
        {
            Ensure(ref labelOffsets, records.Count);
            for (int index = 0; index < records.Count; index++)
            {
                MotionSequenceLabelEvent value = records[index];
                if (string.IsNullOrEmpty(value.Label))
                    throw new InvalidDataException("A Motion sequence label is empty.");
                StringOffset label = builder.CreateString(value.Label);
                Wire.MotionSequenceLabelEvent.StartMotionSequenceLabelEvent(builder);
                Wire.MotionSequenceLabelEvent.AddLabel(builder, label);
                Wire.MotionSequenceLabelEvent.AddGeneration(builder, value.Generation);
                Wire.MotionSequenceLabelEvent.AddPlaybackId(
                    builder,
                    BattlementFlatBufferWriter.WriteUuid(builder, value.PlaybackId.Value)
                );
                labelOffsets[index] = Wire.MotionSequenceLabelEvent.EndMotionSequenceLabelEvent(
                    builder
                );
            }
            Wire.MotionEventBatch.StartLabelEventsVector(builder, records.Count);
            for (int index = records.Count - 1; index >= 0; index--)
                builder.AddOffset(labelOffsets[index].Value);
            return builder.EndVector();
        }

        private Offset<Wire.MotionVector2> Vector(MotionGestureVector value)
        {
            if (!float.IsFinite(value.X) || !float.IsFinite(value.Y))
                throw new InvalidDataException("Motion gesture vectors must be finite.");
            return Wire.MotionVector2.CreateMotionVector2(builder, value.X, value.Y);
        }

        private static (Wire.MotionLifecycleKind Kind, uint First, uint Last) Repeated(
            MotionEventKind.Repeated value
        )
        {
            if (value.First > value.Last)
                throw new InvalidDataException("Motion repeat boundaries must be ordered.");
            return (Wire.MotionLifecycleKind.Repeated, value.First, value.Last);
        }

        private static void ValidateSequence(MotionEventBatch batch)
        {
            if (batch.FirstSequence > batch.LastSequence)
                throw new InvalidDataException("Motion sequence range is reversed.");
            if (batch.Events.Count == 0)
                return;
            if (
                batch.Events[0].Sequence != batch.FirstSequence
                || batch.Events[^1].Sequence != batch.LastSequence
            )
                throw new InvalidDataException("Motion sequence bounds do not match events.");
            for (int index = 1; index < batch.Events.Count; index++)
            {
                ulong previous = batch.Events[index - 1].Sequence;
                if (previous == ulong.MaxValue || batch.Events[index].Sequence != previous + 1)
                    throw new InvalidDataException(
                        "Motion lifecycle sequences must be contiguous."
                    );
            }
        }

        private static void Ensure<T>(ref T[] values, int count)
        {
            if (values.Length < count)
                Array.Resize(ref values, count);
        }

        private static void Limit(int count)
        {
            if (count > MaximumRecords)
                throw new InvalidDataException("A Motion batch has too many records.");
        }
    }
}
