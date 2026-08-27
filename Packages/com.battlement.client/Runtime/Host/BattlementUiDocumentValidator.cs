#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    internal static class BattlementUiDocumentValidator
    {
        public static void Validate(
            GameObjectKind.UiDocumentState value,
            IReadOnlyDictionary<string, PreparedAsset> prepared
        )
        {
            RequireId(value.RootId.Value, "UI root");
            ScreenSize worldSize = value.WorldSpaceSize ?? new ScreenSize(1920, 1080);
            if (worldSize.Width == 0 || worldSize.Height == 0)
                throw Invalid("UI world-space size must be positive.");
            PanelSettingsValue panel = value.PanelSettings ?? new PanelSettingsValue();
            bool screenModeIsSupported =
                panel.RenderMode == PanelRenderMode.ScreenSpaceOverlay
                && value.Position == DocumentPosition.Relative
                && value.WorldSpaceSizeMode == WorldSpaceSizeMode.Fixed;
            bool worldGeometryIsDefault =
                worldSize == new ScreenSize(1920, 1080)
                && value.PivotReferenceSize == PivotReferenceSize.BoundingBox
                && value.Pivot == DocumentPivot.Center;
            if (!screenModeIsSupported || !worldGeometryIsDefault)
            {
                throw Invalid(
                    "World-space UI document settings are not available in this protocol slice."
                );
            }
            RequirePositive(panel.ReferenceSpritePixelsPerUnit, "UI sprite pixels per unit");
            RequirePositive(panel.Scale, "UI panel scale");
            RequirePositive(panel.ReferenceDpi, "UI reference DPI");
            RequirePositive(panel.FallbackDpi, "UI fallback DPI");
            RequireUnit(panel.MatchFactor, "UI match factor");
            if (panel.TargetDisplay > 7)
                throw Invalid("UI target display must be in [0, 7].");
            ValidateTargetTexture(panel, prepared);
            RequireEnum(panel.RenderMode, "panel render mode");
            RequireEnum(panel.ScaleMode, "panel scale mode");
            RequireEnum(panel.ScreenMatchMode, "panel screen match mode");
            ValidateColor(panel.ColorClearValue ?? new Color(0, 0, 0, 0));
            ValidateScaleMode(panel);
            ValidateAtlas(panel.DynamicAtlas ?? new DynamicAtlasSettingsValue());
        }

        private static void ValidateTargetTexture(
            PanelSettingsValue panel,
            IReadOnlyDictionary<string, PreparedAsset> prepared
        )
        {
            if (panel.TargetTexture is not RenderTextureAddress targetTexture)
                return;
            if (string.IsNullOrEmpty(targetTexture.Value))
                throw Invalid("panel target texture address cannot be empty.");
            if (panel.TargetDisplay != 0 || panel.RenderMode != PanelRenderMode.ScreenSpaceOverlay)
            {
                throw Invalid(
                    "A target-texture panel cannot also target a display or use world space."
                );
            }
            if (
                !prepared.TryGetValue(targetTexture.Value, out PreparedAsset asset)
                || asset is not PreparedAsset.RenderTexture
            )
            {
                throw new BattlementWorldException(
                    CoreErrorCode.AssetNotPrepared,
                    $"The panel target texture address '{targetTexture.Value}' was not in the "
                        + "prepared set with the required type."
                );
            }
        }

        private static void ValidateScaleMode(PanelSettingsValue panel)
        {
            if (panel.ScaleMode != PanelScaleMode.ConstantPixelSize && panel.Scale != 1)
                throw Invalid("A nondefault UI panel scale requires constant-pixel scaling.");
            bool dpiIsDefault = panel.ReferenceDpi == 96 && panel.FallbackDpi == 96;
            if (panel.ScaleMode != PanelScaleMode.ConstantPhysicalSize && !dpiIsDefault)
                throw Invalid("Nondefault UI panel DPI requires constant-physical scaling.");
            ScreenSize resolution = panel.ReferenceResolution ?? new ScreenSize(1200, 800);
            if (resolution.Width == 0 || resolution.Height == 0)
                throw Invalid("UI panel reference resolution must be positive.");
            bool referenceIsDefault = resolution == new ScreenSize(1200, 800);
            bool matchingIsDefault =
                panel.ScreenMatchMode == PanelScreenMatchMode.MatchWidthOrHeight
                && panel.MatchFactor == 0;
            if (
                panel.ScaleMode != PanelScaleMode.ScaleWithScreenSize
                && (!referenceIsDefault || !matchingIsDefault)
            )
            {
                throw Invalid(
                    "Reference-resolution settings require scale-with-screen-size scaling."
                );
            }
        }

        private static void ValidateAtlas(DynamicAtlasSettingsValue atlas)
        {
            bool powers = IsPowerOfTwo(atlas.MinAtlasSize) && IsPowerOfTwo(atlas.MaxAtlasSize);
            powers = powers && IsPowerOfTwo(atlas.MaxSubTextureSize);
            bool ordered =
                atlas.MinAtlasSize <= atlas.MaxAtlasSize
                && atlas.MaxSubTextureSize <= atlas.MaxAtlasSize;
            if (!powers || !ordered)
                throw Invalid("UI dynamic atlas sizes must be ordered nonzero powers of two.");
            var filters = new HashSet<DynamicAtlasFilter>();
            foreach (DynamicAtlasFilter filter in atlas.Filters)
            {
                RequireEnum(filter, "dynamic atlas filter");
                if (!filters.Add(filter))
                    throw Invalid("A dynamic atlas filter appeared more than once.");
            }
        }

        private static void ValidateColor(Color value)
        {
            RequireUnit(value.Red, "UI panel clear color red");
            RequireUnit(value.Green, "UI panel clear color green");
            RequireUnit(value.Blue, "UI panel clear color blue");
            RequireUnit(value.Alpha, "UI panel clear color alpha");
        }

        private static void RequireId(Guid value, string name)
        {
            if (value == Guid.Empty)
                throw Invalid($"The {name} UUID must be nonzero.");
        }

        private static void RequirePositive(double value, string name)
        {
            float converted = (float)value;
            if (!double.IsFinite(value) || value <= 0 || !float.IsFinite(converted))
                throw Invalid($"{name} must be finite and positive.");
        }

        private static void RequireUnit(double value, string name)
        {
            float converted = (float)value;
            bool isFinite = double.IsFinite(value) && float.IsFinite(converted);
            if (!isFinite || value is < 0 or > 1)
                throw Invalid($"{name} must be in [0, 1].");
        }

        private static void RequireEnum<T>(T value, string name)
            where T : struct, Enum
        {
            if (!Enum.IsDefined(typeof(T), value))
                throw Invalid($"Unknown {name} value.");
        }

        private static bool IsPowerOfTwo(uint value) => value != 0 && (value & (value - 1)) == 0;

        private static BattlementWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
