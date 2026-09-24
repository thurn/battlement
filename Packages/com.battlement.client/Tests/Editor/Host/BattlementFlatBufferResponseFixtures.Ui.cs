#nullable enable

using System;
using Google.FlatBuffers;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement.Tests
{
    internal static partial class BattlementFlatBufferResponseFixtures
    {
        private static Offset<Wire.PanelSettings> WritePanelSettings(
            FlatBufferBuilder builder,
            PanelSettingsValue value
        )
        {
            DynamicAtlasSettingsValue atlas = value.DynamicAtlas ?? new DynamicAtlasSettingsValue();
            var filters = new byte[atlas.Filters.Count];
            for (int index = 0; index < filters.Length; index++)
                filters[index] = (byte)atlas.Filters[index];
            VectorOffset filterVector = ByteVector(builder, filters);
            Offset<Wire.DynamicAtlasSettings> dynamicAtlas =
                Wire.DynamicAtlasSettings.CreateDynamicAtlasSettings(
                    builder,
                    atlas.MinAtlasSize,
                    atlas.MaxAtlasSize,
                    atlas.MaxSubTextureSize,
                    filterVector
                );
            StringOffset? targetTexture = value.TargetTexture is null
                ? null
                : builder.CreateString(value.TargetTexture.Value.Value);
            ScreenSize resolution = value.ReferenceResolution ?? new ScreenSize(1200, 800);
            Color clearColor = value.ColorClearValue ?? new Color(0, 0, 0, 0);
            Wire.PanelSettings.StartPanelSettings(builder);
            Wire.PanelSettings.AddDynamicAtlas(builder, dynamicAtlas);
            Wire.PanelSettings.AddColorClearValue(builder, Rgba(builder, clearColor));
            Wire.PanelSettings.AddClearColor(builder, value.ClearColor);
            Wire.PanelSettings.AddClearDepthStencil(builder, value.ClearDepthStencil);
            if (targetTexture is StringOffset texture)
                Wire.PanelSettings.AddTargetTexture(builder, texture);
            Wire.PanelSettings.AddTargetDisplay(builder, value.TargetDisplay);
            Wire.PanelSettings.AddMatchFactor(builder, value.MatchFactor);
            Wire.PanelSettings.AddScreenMatchMode(
                builder,
                (Wire.PanelScreenMatchMode)value.ScreenMatchMode
            );
            Wire.PanelSettings.AddReferenceResolution(
                builder,
                Wire.ScreenSize.CreateScreenSize(builder, resolution.Width, resolution.Height)
            );
            Wire.PanelSettings.AddFallbackDpi(builder, value.FallbackDpi);
            Wire.PanelSettings.AddReferenceDpi(builder, value.ReferenceDpi);
            Wire.PanelSettings.AddScale(builder, value.Scale);
            Wire.PanelSettings.AddReferenceSpritePixelsPerUnit(
                builder,
                value.ReferenceSpritePixelsPerUnit
            );
            Wire.PanelSettings.AddScaleMode(builder, (Wire.PanelScaleMode)value.ScaleMode);
            Wire.PanelSettings.AddRenderMode(builder, (Wire.PanelRenderMode)value.RenderMode);
            return Wire.PanelSettings.EndPanelSettings(builder);
        }

        private static Payload VisualCreate(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.Create value
        )
        {
            VectorOffset nodes = WriteNodeVector(builder, new[] { value.Node });
            Wire.VisualElementCreatePayload.StartVisualElementCreatePayload(builder);
            Wire.VisualElementCreatePayload.AddParentId(
                builder,
                Uuid(builder, value.ParentId.Value)
            );
            Wire.VisualElementCreatePayload.AddChildIndex(builder, value.ChildIndex);
            Wire.VisualElementCreatePayload.AddNodes(builder, nodes);
            Wire.VisualElementCreatePayload.AddRootId(
                builder,
                Uuid(builder, value.Node.ObjectId.Value)
            );
            return new(
                Wire.CoreCommandKind.VisualElementCreate,
                Wire.CoreCommandPayload.VisualElementCreatePayload,
                Wire.VisualElementCreatePayload.EndVisualElementCreatePayload(builder).Value
            );
        }

        private static Payload VisualUpdate(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.Update value
        )
        {
            Offset<Wire.UiElement>? element = value.Value is VisualElementUpdate.Properties update
                ? WriteElement(builder, update.Element)
                : null;
            Wire.VisualElementUpdatePayload.StartVisualElementUpdatePayload(builder);
            switch (value.Value)
            {
                case VisualElementUpdate.Properties properties:
                    Wire.VisualElementUpdatePayload.AddKind(
                        builder,
                        Wire.VisualElementUpdateKind.Properties
                    );
                    Wire.VisualElementUpdatePayload.AddObjectId(
                        builder,
                        Uuid(builder, properties.ObjectId.Value)
                    );
                    Wire.VisualElementUpdatePayload.AddElement(builder, element!.Value);
                    break;
                case VisualElementUpdate.Parent parent:
                    Wire.VisualElementUpdatePayload.AddKind(
                        builder,
                        Wire.VisualElementUpdateKind.Parent
                    );
                    Wire.VisualElementUpdatePayload.AddObjectId(
                        builder,
                        Uuid(builder, parent.ObjectId.Value)
                    );
                    Wire.VisualElementUpdatePayload.AddParentId(
                        builder,
                        Uuid(builder, parent.ParentId.Value)
                    );
                    Wire.VisualElementUpdatePayload.AddChildIndex(builder, parent.ChildIndex);
                    break;
                case VisualElementUpdate.Index index:
                    Wire.VisualElementUpdatePayload.AddKind(
                        builder,
                        Wire.VisualElementUpdateKind.Index
                    );
                    Wire.VisualElementUpdatePayload.AddObjectId(
                        builder,
                        Uuid(builder, index.ObjectId.Value)
                    );
                    Wire.VisualElementUpdatePayload.AddChildIndex(builder, index.ChildIndex);
                    break;
                default:
                    throw Unsupported(value);
            }
            return new(
                Wire.CoreCommandKind.VisualElementUpdate,
                Wire.CoreCommandPayload.VisualElementUpdatePayload,
                Wire.VisualElementUpdatePayload.EndVisualElementUpdatePayload(builder).Value
            );
        }

        private static Payload VisualDestroy(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.Destroy value
        )
        {
            Wire.VisualElementDestroyPayload.StartVisualElementDestroyPayload(builder);
            Wire.VisualElementDestroyPayload.AddObjectId(
                builder,
                Uuid(builder, value.ObjectId.Value)
            );
            return new(
                Wire.CoreCommandKind.VisualElementDestroy,
                Wire.CoreCommandPayload.VisualElementDestroyPayload,
                Wire.VisualElementDestroyPayload.EndVisualElementDestroyPayload(builder).Value
            );
        }

        private static Payload VisualAction(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.PerformAction value
        ) => throw Unsupported(value);

        private static Payload MotionValue(
            FlatBufferBuilder builder,
            CommandBody.Motion.ValueCommand value
        ) => throw Unsupported(value);

        private static Payload MotionValuePlayback(
            FlatBufferBuilder builder,
            CommandBody.Motion.ValuePlayback value
        ) => throw Unsupported(value);

        private static Payload MotionPlayback(
            FlatBufferBuilder builder,
            CommandBody.Motion.Playback value
        ) => throw Unsupported(value);

        private static Payload MotionControlledClock(
            FlatBufferBuilder builder,
            CommandBody.Motion.ControlledClock value
        ) => throw Unsupported(value);

        private static Payload MotionControl(
            FlatBufferBuilder builder,
            CommandBody.Motion.Control value
        )
        {
            if (
                value.Payload.Command is not MotionControlCommand.Start start
                || start.Target is not MotionControlTarget.Variant variant
            )
                throw Unsupported(value);
            StringOffset name = builder.CreateString(variant.Value);
            Offset<Wire.MotionControlTarget> target =
                Wire.MotionControlTarget.CreateMotionControlTarget(
                    builder,
                    Wire.MotionControlTargetKind.Variant,
                    default,
                    name
                );
            Wire.MotionControlOperation.StartMotionControlOperation(builder);
            Wire.MotionControlOperation.AddTarget(builder, target);
            Wire.MotionControlOperation.AddGeneration(builder, start.Generation);
            Wire.MotionControlOperation.AddPlaybackId(
                builder,
                Uuid(builder, start.PlaybackId.Value)
            );
            Wire.MotionControlOperation.AddCommand(builder, Wire.MotionControlCommandKind.Start);
            Wire.MotionControlOperation.AddControlId(
                builder,
                Uuid(builder, value.Payload.ControlId.Value)
            );
            return new Payload(
                Wire.CoreCommandKind.MotionControl,
                Wire.CoreCommandPayload.MotionControlOperation,
                Wire.MotionControlOperation.EndMotionControlOperation(builder).Value
            );
        }

        private static Payload MotionScope(
            FlatBufferBuilder builder,
            CommandBody.Motion.Scope value
        )
        {
            if (value.Payload.Command is not MotionScopeCommand.Start start)
                throw Unsupported(value);
            var entries = new int[start.Entries.Count];
            for (int index = 0; index < entries.Length; index++)
            {
                if (
                    start.Entries[index] is not MotionSequenceEntry.Particle particle
                    || particle.Schedule is not MotionSequenceSchedule.Absolute schedule
                )
                    throw Unsupported(value);
                MotionPositionReference position = particle.Occurrence.Position;
                StringOffset? anchor = position.Anchor is null
                    ? null
                    : builder.CreateString(position.Anchor);
                Wire.MotionPositionReference.StartMotionPositionReference(builder);
                Wire.MotionPositionReference.AddResolution(
                    builder,
                    (Wire.MotionReferenceResolution)position.Resolution
                );
                Wire.MotionPositionReference.AddOffset(
                    builder,
                    Wire.MotionVector3.CreateMotionVector3(
                        builder,
                        (float)position.Offset.X,
                        (float)position.Offset.Y,
                        (float)position.Offset.Z
                    )
                );
                if (anchor is StringOffset anchorValue)
                    Wire.MotionPositionReference.AddAnchor(builder, anchorValue);
                Wire.MotionPositionReference.AddObjectId(
                    builder,
                    Uuid(builder, position.ObjectId.Value)
                );
                Offset<Wire.MotionPositionReference> reference =
                    Wire.MotionPositionReference.EndMotionPositionReference(builder);
                Offset<Wire.MotionSequenceSchedule> wireSchedule =
                    Wire.MotionSequenceSchedule.CreateMotionSequenceSchedule(
                        builder,
                        Wire.MotionSequenceScheduleKind.Absolute,
                        absolute_micros: schedule.StartMicros
                    );
                StringOffset address = builder.CreateString(particle.Occurrence.Address);
                entries[index] = Wire
                    .MotionSequenceEntry.CreateMotionSequenceEntry(
                        builder,
                        Wire.MotionSequenceEntryKind.Particle,
                        scheduleOffset: wireSchedule,
                        effect_addressOffset: address,
                        effect_positionOffset: reference,
                        effect_lifetime_millis: particle.Occurrence.LifetimeMilliseconds
                    )
                    .Value;
            }
            VectorOffset entryVector = OffsetVector(builder, entries);
            Wire.MotionScopeOperation.StartMotionScopeOperation(builder);
            Wire.MotionScopeOperation.AddEntries(builder, entryVector);
            Wire.MotionScopeOperation.AddGeneration(builder, start.Generation);
            Wire.MotionScopeOperation.AddPlaybackId(builder, Uuid(builder, start.PlaybackId.Value));
            Wire.MotionScopeOperation.AddCommand(builder, Wire.MotionScopeCommandKind.Start);
            Wire.MotionScopeOperation.AddScopeId(
                builder,
                Uuid(builder, value.Payload.ScopeId.Value)
            );
            return new Payload(
                Wire.CoreCommandKind.MotionScope,
                Wire.CoreCommandPayload.MotionScopeOperation,
                Wire.MotionScopeOperation.EndMotionScopeOperation(builder).Value
            );
        }

        private static Payload MotionDragControl(
            FlatBufferBuilder builder,
            CommandBody.Motion.DragControl value
        ) => throw Unsupported(value);

        private static NotSupportedException Unsupported(object value) =>
            new($"The host fixture writer cannot encode {value.GetType().FullName}.");
    }
}
