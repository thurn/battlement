#nullable enable

using System;
using System.IO;
using Wire = Battlement.FlatBuffers.Generated;

namespace Battlement
{
    /// <summary>
    /// Scalar adapters used while the host applies verified FlatBuffer views.
    /// </summary>
    internal static partial class BattlementFlatBufferRetainedCopy
    {
        internal static Google.FlatBuffers.ByteBuffer Open(ReadOnlyMemory<byte> payload) =>
            new(new BattlementReadOnlyByteBufferAllocator(payload), 0);

        internal static GameObjectKind.UiDocumentState UiDocumentState(Wire.UiDocumentObject value)
        {
            Wire.PanelSettings panel = value.PanelSettings ?? throw Missing("panel settings");
            Wire.DynamicAtlasSettings atlas =
                panel.DynamicAtlas ?? throw Missing("dynamic atlas settings");
            var filters = new DynamicAtlasFilter[atlas.FiltersLength];
            for (int index = 0; index < filters.Length; index++)
                filters[index] = (DynamicAtlasFilter)(byte)atlas.Filters(index);
            Wire.ScreenSize referenceResolution =
                panel.ReferenceResolution ?? throw Missing("panel reference resolution");
            Wire.ScreenSize worldSpaceSize =
                value.WorldSpaceSize ?? throw Missing("world-space document size");
            return new GameObjectKind.UiDocumentState(
                new ObjectId(Uuid(value.RootId, "UI document root")),
                new PanelSettingsValue(
                    (PanelRenderMode)(byte)panel.RenderMode,
                    (PanelScaleMode)(byte)panel.ScaleMode,
                    panel.ReferenceSpritePixelsPerUnit,
                    panel.Scale,
                    panel.ReferenceDpi,
                    panel.FallbackDpi,
                    new ScreenSize(referenceResolution.Width, referenceResolution.Height),
                    (PanelScreenMatchMode)(byte)panel.ScreenMatchMode,
                    panel.MatchFactor,
                    panel.TargetDisplay,
                    panel.TargetTexture is null
                        ? null
                        : new RenderTextureAddress(panel.TargetTexture),
                    panel.ClearDepthStencil,
                    panel.ClearColor,
                    Rgba(panel.ColorClearValue ?? throw Missing("panel clear color")),
                    new DynamicAtlasSettingsValue(
                        atlas.MinAtlasSize,
                        atlas.MaxAtlasSize,
                        atlas.MaxSubTextureSize,
                        filters
                    )
                ),
                (DocumentPosition)(byte)value.Position,
                (WorldSpaceSizeMode)(byte)value.WorldSpaceSizeMode,
                new ScreenSize(worldSpaceSize.Width, worldSpaceSize.Height),
                (PivotReferenceSize)(byte)value.PivotReferenceSize,
                (DocumentPivot)(byte)value.Pivot,
                value.SortingOrder
            );
        }

        private static Color Rgba(Wire.RgbaColor value) => new(value.R, value.G, value.B, value.A);

        private static ObjectId ObjectId(Wire.Uuid? value) => new(Uuid(value, "object"));

        private static ObjectId? OptionalObjectId(Wire.Uuid? value) =>
            value.HasValue ? new ObjectId(Uuid(value, "object")) : null;

        private static Guid Uuid(Wire.Uuid? value, string field) =>
            BattlementFlatBufferResponse.ReadUuid(value, field);

        private static string Required(string? value, string field) =>
            value ?? throw new InvalidDataException($"The {field} string is absent.");

        private static InvalidDataException Missing(string field) =>
            new($"FlatBuffer {field} is absent.");

        private static InvalidDataException Unsupported(string value) =>
            new($"FlatBuffer {value} is not implemented by the host reader.");
    }
}
