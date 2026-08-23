#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using ProtocolFlexDirection = Battlement.UiFlexDirection;
using ProtocolPanelScaleMode = Battlement.PanelScaleMode;
using ProtocolScreenMatchMode = Battlement.PanelScreenMatchMode;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityPanelScaleMode = UnityEngine.UIElements.PanelScaleMode;
using UnityScreenMatchMode = UnityEngine.UIElements.PanelScreenMatchMode;

namespace Battlement.UI
{
    /// <summary>Constructs and populates Battlement-owned UI Toolkit documents.</summary>
    public sealed class BattlementUiDocuments
    {
        private readonly Dictionary<Guid, UnityEngine.UIElements.VisualElement> elements = new();

        /// <summary>Creates an empty native UI-document GameObject.</summary>
        public static GameObject CreateGameObject(GameObjectKind.UiDocumentState description)
        {
            var gameObject = new GameObject("Battlement UI Document");
            UIDocument document = gameObject.AddComponent<UIDocument>();
            // A bare CreateInstance<PanelSettings>() does not reliably initialize Unity's
            // internal UI resource graph in a packaged player. Loading a serialized template
            // makes Unity retain and resolve those runtime dependencies. Clone the template
            // because every Battlement document owns mutable panel settings.
            UnityEngine.UIElements.PanelSettings template =
                Resources.Load<UnityEngine.UIElements.PanelSettings>(
                    "BattlementPanelSettingsTemplate"
                );
            if (template == null)
            {
                throw new InvalidOperationException(
                    "Battlement panel settings template is missing."
                );
            }
            UnityEngine.UIElements.PanelSettings panel = Object.Instantiate(template);
            panel.name = "Battlement Runtime Panel";
            ApplyPanelSettings(panel, description.PanelSettings ?? new PanelSettingsValue());
            document.panelSettings = panel;
            document.sortingOrder = description.SortingOrder;
            gameObject.AddComponent<BattlementUiDocumentOwner>().Initialize(panel);
            return gameObject;
        }

        /// <summary>Replaces tracked hierarchies from an authoritative snapshot.</summary>
        public void Replace(
            IReadOnlyList<UiDocument>? descriptions,
            Func<ObjectId, GameObject?> resolveGameObject
        )
        {
            elements.Clear();
            foreach (UiDocument description in descriptions ?? Array.Empty<UiDocument>())
            {
                GameObject? gameObject = resolveGameObject(description.DocumentId);
                if (gameObject == null)
                {
                    throw new InvalidOperationException(
                        $"UI document {description.DocumentId} has no owning GameObject."
                    );
                }
                if (!gameObject.TryGetComponent(out UIDocument document))
                {
                    throw new InvalidOperationException(
                        $"UI document {description.DocumentId} has no UIDocument component."
                    );
                }
                UnityEngine.UIElements.VisualElement root = document.rootVisualElement;
                root.Clear();
                ApplyCommon(
                    root,
                    description.Name,
                    description.Enabled,
                    description.Classes,
                    description.Style
                );
                Reserve(description.RootId, root);
                foreach (UiElement child in description.Children ?? Array.Empty<UiElement>())
                {
                    root.Add(CreateElement(child));
                }
            }
        }

        /// <summary>Finds a tracked document root or authored element.</summary>
        public bool TryGet(ObjectId objectId, out UnityEngine.UIElements.VisualElement? value) =>
            elements.TryGetValue(objectId.Value, out value);

        /// <summary>Gets the identities currently owned by UI Toolkit elements.</summary>
        public IEnumerable<Guid> IdentityIds => elements.Keys;

        /// <summary>Releases every tracked root and element identity.</summary>
        public void Clear() => elements.Clear();

        private UnityEngine.UIElements.VisualElement CreateElement(UiElement description)
        {
            UnityEngine.UIElements.VisualElement value = description switch
            {
                UiElement.VisualElement => new UnityEngine.UIElements.VisualElement(),
                UiElement.Box => new UnityEngine.UIElements.Box(),
                UiElement.Label label => new UnityEngine.UIElements.Label(
                    label.Text ?? string.Empty
                ),
                _ => throw new InvalidOperationException("Unsupported UI element type."),
            };

            switch (description)
            {
                case UiElement.VisualElement element:
                    Populate(
                        value,
                        element.ObjectId,
                        element.Name,
                        element.Enabled,
                        element.Classes,
                        element.Style,
                        element.Children
                    );
                    break;
                case UiElement.Box box:
                    Populate(
                        value,
                        box.ObjectId,
                        box.Name,
                        box.Enabled,
                        box.Classes,
                        box.Style,
                        box.Children
                    );
                    break;
                case UiElement.Label label:
                    Populate(
                        value,
                        label.ObjectId,
                        label.Name,
                        label.Enabled,
                        label.Classes,
                        label.Style,
                        label.Children
                    );
                    if ((label.Children?.Count ?? 0) != 0)
                    {
                        throw new InvalidOperationException("Label elements cannot have children.");
                    }
                    break;
                default:
                    throw new InvalidOperationException("Unsupported UI element type.");
            }
            return value;
        }

        private void Populate(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId,
            string? name,
            bool enabled,
            IReadOnlyList<string>? classes,
            UiStyle? style,
            IReadOnlyList<UiElement>? children
        )
        {
            ApplyCommon(value, name, enabled, classes, style);
            Reserve(objectId, value);
            foreach (UiElement child in children ?? Array.Empty<UiElement>())
            {
                value.Add(CreateElement(child));
            }
        }

        private void Reserve(ObjectId objectId, UnityEngine.UIElements.VisualElement value)
        {
            if (!elements.TryAdd(objectId.Value, value))
            {
                throw new InvalidOperationException($"UI identity {objectId} is duplicated.");
            }
        }

        private static void ApplyCommon(
            UnityEngine.UIElements.VisualElement value,
            string? name,
            bool enabled,
            IReadOnlyList<string>? classes,
            UiStyle? style
        )
        {
            value.name = name ?? string.Empty;
            value.SetEnabled(enabled);
            foreach (string className in classes ?? Array.Empty<string>())
            {
                value.AddToClassList(className);
            }
            ApplyStyle(value.style, style);
        }

        private static void ApplyStyle(IStyle target, UiStyle? value)
        {
            if (value is null)
            {
                return;
            }
            if (value.BackgroundColor is Color background)
            {
                target.backgroundColor = ToUnity(background);
            }
            if (value.Color is Color foreground)
            {
                target.color = ToUnity(foreground);
            }
            if (value.Width is float width)
                target.width = width;
            if (value.Height is float height)
                target.height = height;
            if (value.FlexGrow is float flexGrow)
                target.flexGrow = flexGrow;
            if (value.FlexDirection is ProtocolFlexDirection direction)
            {
                target.flexDirection =
                    direction == ProtocolFlexDirection.Row
                        ? UnityFlexDirection.Row
                        : UnityFlexDirection.Column;
            }
            if (value.Padding is float padding)
            {
                target.paddingTop = padding;
                target.paddingRight = padding;
                target.paddingBottom = padding;
                target.paddingLeft = padding;
            }
            if (value.Margin is float margin)
            {
                target.marginTop = margin;
                target.marginRight = margin;
                target.marginBottom = margin;
                target.marginLeft = margin;
            }
            if (value.FontSize is float fontSize)
                target.fontSize = fontSize;
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

    // UIDocument does not destroy a runtime PanelSettings clone assigned to it. Keep the clone
    // on a companion component so ordinary GameObject teardown releases the native resource.
    [ExecuteAlways]
    internal sealed class BattlementUiDocumentOwner : MonoBehaviour
    {
        private UnityEngine.UIElements.PanelSettings? panel;

        public void Initialize(UnityEngine.UIElements.PanelSettings value) => panel = value;

        private void OnDestroy()
        {
            if (panel == null)
            {
                return;
            }
            if (Application.isPlaying)
                Object.Destroy(panel);
            else
                Object.DestroyImmediate(panel);
            panel = null;
        }
    }
}
