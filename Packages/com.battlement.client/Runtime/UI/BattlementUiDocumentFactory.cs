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
        public static GameObject Create(
            GameObjectKind.UiDocumentState description,
            IBattlementUiAssetLookup? assets
        )
        {
            var gameObject = new GameObject("Battlement UI Document");
            UnityEngine.UIElements.PanelSettings? panel = null;
            IBattlementUiAssetLease? targetTexture = null;
            try
            {
                UIDocument document = gameObject.AddComponent<UIDocument>();
                UnityEngine.UIElements.PanelSettings template =
                    Resources.Load<UnityEngine.UIElements.PanelSettings>(
                        "BattlementPanelSettingsTemplate"
                    );
                if (template == null)
                    throw new InvalidOperationException(
                        "Battlement panel settings template is missing."
                    );
                panel = Object.Instantiate(template);
                panel.name = "Battlement Runtime Panel";
                PanelTextSettings textSettings = Resources.Load<PanelTextSettings>(
                    "BattlementTextSettings"
                );
                if (textSettings != null)
                    panel.textSettings = textSettings;
                PanelSettingsValue settings = description.PanelSettings ?? new PanelSettingsValue();
                ApplyPanelSettings(panel, settings);
                if (settings.TargetTexture is RenderTextureAddress address)
                {
                    targetTexture =
                        assets?.Acquire(new PreparedAsset.RenderTexture(address))
                        ?? throw new InvalidOperationException(
                            "A target-texture panel requires prepared asset access."
                        );
                    if (targetTexture.Value is not RenderTexture texture)
                    {
                        throw new InvalidOperationException(
                            "A panel target texture resolved to the wrong Unity type."
                        );
                    }
                    panel.targetTexture = texture;
                }
                document.panelSettings = panel;
                document.sortingOrder = description.SortingOrder;
                gameObject
                    .AddComponent<BattlementUiDocumentOwner>()
                    .Initialize(panel, targetTexture);
                targetTexture = null;
                return gameObject;
            }
            catch
            {
                targetTexture?.Dispose();
                if (panel != null)
                    Object.DestroyImmediate(panel);
                Object.DestroyImmediate(gameObject);
                throw;
            }
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
        private IBattlementUiAssetLease? targetTexture;

        public void Initialize(
            UnityEngine.UIElements.PanelSettings value,
            IBattlementUiAssetLease? texture
        )
        {
            panel = value;
            targetTexture = texture;
        }

        private void OnDestroy()
        {
            if (panel != null)
            {
                if (Application.isPlaying)
                    Object.Destroy(panel);
                else
                    Object.DestroyImmediate(panel);
            }
            panel = null;
            targetTexture?.Dispose();
            targetTexture = null;
        }
    }
}
