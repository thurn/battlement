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
        ) => throw Unsupported(value);

        private static Payload VisualUpdate(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.Update value
        ) => throw Unsupported(value);

        private static Payload VisualDestroy(
            FlatBufferBuilder builder,
            CommandBody.VisualElement.Destroy value
        ) => throw Unsupported(value);

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
        ) => throw Unsupported(value);

        private static Payload MotionScope(
            FlatBufferBuilder builder,
            CommandBody.Motion.Scope value
        ) => throw Unsupported(value);

        private static Payload MotionDragControl(
            FlatBufferBuilder builder,
            CommandBody.Motion.DragControl value
        ) => throw Unsupported(value);

        private static NotSupportedException Unsupported(object value) =>
            new($"The host fixture writer cannot encode {value.GetType().FullName}.");
    }
}
