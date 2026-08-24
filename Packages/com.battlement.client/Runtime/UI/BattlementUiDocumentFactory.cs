#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using ProtocolPanelScaleMode = Battlement.PanelScaleMode;
using ProtocolScreenMatchMode = Battlement.PanelScreenMatchMode;
using UnityPanelScaleMode = UnityEngine.UIElements.PanelScaleMode;
using UnityScreenMatchMode = UnityEngine.UIElements.PanelScreenMatchMode;

namespace Battlement.UI
{
    internal static class BattlementUiDocumentFactory
    {
        public static GameObject Create(GameObjectKind.UiDocumentState description)
        {
            var gameObject = new GameObject("Battlement UI Document");
            UIDocument document = gameObject.AddComponent<UIDocument>();
            UnityEngine.UIElements.PanelSettings template =
                Resources.Load<UnityEngine.UIElements.PanelSettings>(
                    "BattlementPanelSettingsTemplate"
                );
            if (template == null)
                throw new InvalidOperationException(
                    "Battlement panel settings template is missing."
                );
            UnityEngine.UIElements.PanelSettings panel = Object.Instantiate(template);
            panel.name = "Battlement Runtime Panel";
            ApplyPanelSettings(panel, description.PanelSettings ?? new PanelSettingsValue());
            document.panelSettings = panel;
            document.sortingOrder = description.SortingOrder;
            gameObject.AddComponent<BattlementUiDocumentOwner>().Initialize(panel);
            return gameObject;
        }

        private static void ApplyPanelSettings(
            UnityEngine.UIElements.PanelSettings target,
            PanelSettingsValue value
        )
        {
            target.scaleMode = value.ScaleMode switch
            {
                ProtocolPanelScaleMode.ConstantPixelSize => UnityPanelScaleMode.ConstantPixelSize,
                ProtocolPanelScaleMode.ScaleWithScreenSize =>
                    UnityPanelScaleMode.ScaleWithScreenSize,
                _ => UnityPanelScaleMode.ConstantPhysicalSize,
            };
            target.referenceSpritePixelsPerUnit = value.ReferenceSpritePixelsPerUnit;
            target.scale = value.Scale;
            target.referenceDpi = value.ReferenceDpi;
            target.fallbackDpi = value.FallbackDpi;
            ScreenSize resolution = value.ReferenceResolution ?? new ScreenSize(1200, 800);
            target.referenceResolution = new Vector2Int(
                (int)resolution.Width,
                (int)resolution.Height
            );
            target.screenMatchMode = value.ScreenMatchMode switch
            {
                ProtocolScreenMatchMode.MatchWidthOrHeight =>
                    UnityScreenMatchMode.MatchWidthOrHeight,
                ProtocolScreenMatchMode.Shrink => UnityScreenMatchMode.Shrink,
                _ => UnityScreenMatchMode.Expand,
            };
            target.match = value.MatchFactor;
            target.targetDisplay = (int)value.TargetDisplay;
            target.clearDepthStencil = value.ClearDepthStencil;
            target.clearColor = value.ClearColor;
            target.colorClearValue = ToUnity(value.ColorClearValue ?? new Color(0, 0, 0, 0));
            DynamicAtlasSettingsValue atlas = value.DynamicAtlas ?? new DynamicAtlasSettingsValue();
            target.dynamicAtlasSettings = new UnityEngine.UIElements.DynamicAtlasSettings
            {
                minAtlasSize = (int)atlas.MinAtlasSize,
                maxAtlasSize = (int)atlas.MaxAtlasSize,
                maxSubTextureSize = (int)atlas.MaxSubTextureSize,
                activeFilters = AtlasFilters(atlas.Filters),
            };
        }

        private static DynamicAtlasFilters AtlasFilters(IReadOnlyList<DynamicAtlasFilter> values)
        {
            DynamicAtlasFilters result = DynamicAtlasFilters.None;
            foreach (DynamicAtlasFilter value in values)
            {
                result |= value switch
                {
                    DynamicAtlasFilter.Readability => DynamicAtlasFilters.Readability,
                    DynamicAtlasFilter.Size => DynamicAtlasFilters.Size,
                    DynamicAtlasFilter.Format => DynamicAtlasFilters.Format,
                    DynamicAtlasFilter.ColorSpace => DynamicAtlasFilters.ColorSpace,
                    _ => DynamicAtlasFilters.FilterMode,
                };
            }
            return result;
        }

        private static UnityEngine.Color ToUnity(Color value) =>
            new((float)value.Red, (float)value.Green, (float)value.Blue, (float)value.Alpha);
    }

    [ExecuteAlways]
    internal sealed class BattlementUiDocumentOwner : MonoBehaviour
    {
        private UnityEngine.UIElements.PanelSettings? panel;

        public void Initialize(UnityEngine.UIElements.PanelSettings value) => panel = value;

        private void OnDestroy()
        {
            if (panel == null)
                return;
            if (Application.isPlaying)
                Object.Destroy(panel);
            else
                Object.DestroyImmediate(panel);
            panel = null;
        }
    }
}
