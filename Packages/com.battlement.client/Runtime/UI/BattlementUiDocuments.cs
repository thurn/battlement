#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using ProtocolFlexDirection = Battlement.UiFlexDirection;
using ProtocolPanelScaleMode = Battlement.PanelScaleMode;
using ProtocolScreenMatchMode = Battlement.PanelScreenMatchMode;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityPanelScaleMode = UnityEngine.UIElements.PanelScaleMode;
using UnityScreenMatchMode = UnityEngine.UIElements.PanelScreenMatchMode;

namespace Battlement.UI
{
    /// <summary>Constructs and populates Battlement-owned UI Toolkit documents.</summary>
    public sealed class BattlementUiDocuments
    {
        private readonly Dictionary<Guid, UnityEngine.UIElements.VisualElement> elements = new();
        private readonly Dictionary<Guid, HashSet<string>> authoredClasses = new();
        private readonly Dictionary<Guid, HashSet<UiEventKind>> subscriptions = new();
        private readonly HashSet<Guid> rootIds = new();
        private readonly Func<UiEvent, bool>? emit;

        /// <summary>Creates a document manager with an optional synchronous event sink.</summary>
        public BattlementUiDocuments(Func<UiEvent, bool>? emitUiEvent = null) => emit = emitUiEvent;

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
            authoredClasses.Clear();
            subscriptions.Clear();
            rootIds.Clear();
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
                    description.RootId,
                    description.Name,
                    description.Enabled,
                    description.Classes,
                    description.Style,
                    description.Events
                );
                Reserve(description.RootId, root);
                rootIds.Add(description.RootId.Value);
                foreach (UiNode child in description.Children ?? Array.Empty<UiNode>())
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
        public void Clear()
        {
            elements.Clear();
            authoredClasses.Clear();
            subscriptions.Clear();
            rootIds.Clear();
        }

        /// <summary>Creates and attaches one validated element subtree.</summary>
        public void Create(CommandBody.VisualElement.Create command)
        {
            UnityEngine.UIElements.VisualElement parent = Require(command.ParentId);
            RequireContainer(parent, command.ParentId);
            int index = command.ChildIndex is uint requested
                ? checked((int)requested)
                : parent.childCount;
            if (index > parent.childCount)
            {
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            }

            var ids = new HashSet<Guid>();
            ValidateDetached(command.Node, ids);
            try
            {
                UnityEngine.UIElements.VisualElement created = CreateElement(command.Node);
                if (command.ChildIndex is null)
                    parent.Add(created);
                else
                    parent.Insert(index, created);
            }
            catch
            {
                foreach (Guid id in ids)
                {
                    elements.Remove(id);
                    authoredClasses.Remove(id);
                    subscriptions.Remove(id);
                }
                throw;
            }
        }

        /// <summary>Applies one sparse property or hierarchy update.</summary>
        public void Update(CommandBody.VisualElement.Update command)
        {
            switch (command.Value)
            {
                case VisualElementUpdate.Properties properties:
                    UnityEngine.UIElements.VisualElement target = Require(properties.ObjectId);
                    RequireElementKind(target, properties.Element, properties.ObjectId);
                    ApplyElementValues(target, properties.ObjectId, properties.Element);
                    break;
                case VisualElementUpdate.Parent parent:
                    ApplyParent(Require(parent.ObjectId), parent.ObjectId, parent.ParentId);
                    break;
                case VisualElementUpdate.Index index:
                    ApplyIndex(Require(index.ObjectId), index.ObjectId, index.ChildIndex);
                    break;
                default:
                    throw new InvalidOperationException("Unsupported UI update type.");
            }
        }

        /// <summary>Destroys one non-root element and its logical descendants.</summary>
        public void Destroy(CommandBody.VisualElement.Destroy command)
        {
            UnityEngine.UIElements.VisualElement target = Require(command.ObjectId);
            if (rootIds.Contains(command.ObjectId.Value))
            {
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be destroyed by a UI command."
                );
            }
            var removed = new List<Guid>();
            foreach (KeyValuePair<Guid, UnityEngine.UIElements.VisualElement> entry in elements)
            {
                if (entry.Value == target || target.Contains(entry.Value))
                    removed.Add(entry.Key);
            }
            target.RemoveFromHierarchy();
            foreach (Guid id in removed)
            {
                elements.Remove(id);
                authoredClasses.Remove(id);
                subscriptions.Remove(id);
            }
        }

        /// <summary>Rejects native-only actions that this executor does not simulate.</summary>
        public void PerformAction(CommandBody.VisualElement.PerformAction command) =>
            throw Failure(
                CoreErrorCode.InvalidProperty,
                $"UI action {command.Action.GetType().Name} is unsupported by this executor."
            );

        private UnityEngine.UIElements.VisualElement CreateElement(UiNode node)
        {
            UiElement description = node.Element;
            UnityEngine.UIElements.VisualElement value = description switch
            {
                UiElement.VisualElement => new UnityEngine.UIElements.VisualElement(),
                UiElement.Box => new UnityEngine.UIElements.Box(),
                UiElement.Label label => new UnityEngine.UIElements.Label(
                    label.Text ?? string.Empty
                ),
                UiElement.Button button => new UnityEngine.UIElements.Button
                {
                    text = button.Text ?? string.Empty,
                },
                _ => throw new InvalidOperationException("Unsupported UI element type."),
            };

            Populate(value, node);
            if (description is UiElement.Button)
                value.RegisterCallback<UnityClickEvent>(eventValue =>
                    ForwardClick(node.ObjectId, eventValue)
                );
            return value;
        }

        private void Populate(UnityEngine.UIElements.VisualElement value, UiNode node)
        {
            ApplyCommon(
                value,
                node.ObjectId,
                node.Element.Name,
                node.Element.Enabled,
                node.Element.Classes,
                node.Element.Style,
                node.Element.Events
            );
            Reserve(node.ObjectId, value);
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
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

        private void ApplyCommon(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId,
            string? name,
            bool? enabled,
            IReadOnlyList<string>? classes,
            UiStyle? style,
            IReadOnlyList<UiEventKind>? events
        )
        {
            if (name is not null)
                value.name = name;
            if (enabled is bool enabledValue)
                value.SetEnabled(enabledValue);
            var classSet = new HashSet<string>();
            foreach (string className in classes ?? Array.Empty<string>())
            {
                value.AddToClassList(className);
                classSet.Add(className);
            }
            authoredClasses[objectId.Value] = classSet;
            ApplyStyle(value.style, style);
            subscriptions[objectId.Value] = new HashSet<UiEventKind>(
                events ?? Array.Empty<UiEventKind>()
            );
        }

        private void ForwardClick(ObjectId objectId, UnityClickEvent eventValue)
        {
            if (
                emit is null
                || !subscriptions.TryGetValue(objectId.Value, out HashSet<UiEventKind> values)
                || !values.Contains(UiEventKind.Click)
            )
            {
                return;
            }

            emit(
                new UiEvent(
                    objectId,
                    new UiEventBody.Click(
                        new Battlement.ClickEvent.Pointer(
                            new PanelPoint(eventValue.position.x, eventValue.position.y),
                            checked((uint)Math.Max(1, eventValue.clickCount)),
                            eventValue.pointerId,
                            ToPointerButton(eventValue.button),
                            ToModifiers(eventValue.modifiers)
                        )
                    )
                )
            );
        }

        private static PointerButton ToPointerButton(int value) =>
            value switch
            {
                1 => PointerButton.Right,
                2 => PointerButton.Middle,
                _ => PointerButton.Left,
            };

        private static IReadOnlyList<KeyModifier> ToModifiers(EventModifiers values)
        {
            var result = new List<KeyModifier>();
            if ((values & EventModifiers.Alt) != 0)
                result.Add(KeyModifier.Alt);
            if ((values & EventModifiers.Control) != 0)
                result.Add(KeyModifier.Control);
            if ((values & EventModifiers.Command) != 0)
                result.Add(KeyModifier.Command);
            if ((values & EventModifiers.Shift) != 0)
                result.Add(KeyModifier.Shift);
            return result;
        }

        private UnityEngine.UIElements.VisualElement Require(ObjectId objectId)
        {
            if (
                !elements.TryGetValue(
                    objectId.Value,
                    out UnityEngine.UIElements.VisualElement value
                )
            )
            {
                throw Failure(
                    CoreErrorCode.UnknownObject,
                    $"UI element {objectId} does not exist."
                );
            }
            return value;
        }

        private static void RequireContainer(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId
        )
        {
            if (value is UnityEngine.UIElements.Label or UnityEngine.UIElements.Button)
            {
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    $"UI element {objectId} cannot contain children."
                );
            }
        }

        private void ValidateDetached(UiNode node, ISet<Guid> ids)
        {
            if (!ids.Add(node.ObjectId.Value) || elements.ContainsKey(node.ObjectId.Value))
                throw Failure(
                    CoreErrorCode.DuplicateId,
                    $"UI identity {node.ObjectId} is duplicated."
                );
            IReadOnlyList<UiNode> children = node.Children ?? Array.Empty<UiNode>();
            if (node.Element is UiElement.Label or UiElement.Button && children.Count != 0)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Leaf UI controls cannot contain logical children."
                );
            foreach (UiNode child in children)
                ValidateDetached(child, ids);
        }

        private static void RequireElementKind(
            UnityEngine.UIElements.VisualElement target,
            UiElement element,
            ObjectId objectId
        )
        {
            bool matches = element switch
            {
                UiElement.VisualElement => target.GetType()
                    == typeof(UnityEngine.UIElements.VisualElement),
                UiElement.Box => target.GetType() == typeof(UnityEngine.UIElements.Box),
                UiElement.Label => target.GetType() == typeof(UnityEngine.UIElements.Label),
                UiElement.Button => target.GetType() == typeof(UnityEngine.UIElements.Button),
                _ => false,
            };
            if (!matches)
                throw new InvalidOperationException(
                    $"UI element {objectId} update has the wrong concrete class."
                );
        }

        private void ApplyElementValues(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement element
        )
        {
            if (element.Name is string name)
                target.name = name;
            if (element.Enabled is bool enabled)
                target.SetEnabled(enabled);
            if (element.Classes is IReadOnlyList<string> classes)
            {
                foreach (string value in authoredClasses[objectId.Value])
                    target.RemoveFromClassList(value);
                var replacements = new HashSet<string>();
                foreach (string value in classes)
                {
                    target.AddToClassList(value);
                    replacements.Add(value);
                }
                authoredClasses[objectId.Value] = replacements;
            }
            ApplyStyle(target.style, element.Style);
            if (element.Events is IReadOnlyList<UiEventKind> events)
                subscriptions[objectId.Value] = new HashSet<UiEventKind>(events);
            switch (element)
            {
                case UiElement.Label label when label.Text is string text:
                    ((UnityEngine.UIElements.Label)target).text = text;
                    break;
                case UiElement.Button button when button.Text is string text:
                    ((UnityEngine.UIElements.Button)target).text = text;
                    break;
                default:
                    break;
            }
        }

        private void ApplyParent(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            ObjectId parentId
        )
        {
            if (rootIds.Contains(objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be reparented."
                );
            UnityEngine.UIElements.VisualElement parent = Require(parentId);
            RequireContainer(parent, parentId);
            if (target == parent || target.Contains(parent))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A UI placement cannot create a cycle."
                );
            target.RemoveFromHierarchy();
            parent.Add(target);
        }

        private static void ApplyIndex(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            uint childIndex
        )
        {
            UnityEngine.UIElements.VisualElement? parent = target.parent;
            if (parent is null)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    $"UI element {objectId} has no logical parent."
                );
            int index = checked((int)childIndex);
            if (index >= parent.childCount)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            target.RemoveFromHierarchy();
            parent.Insert(index, target);
        }

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);

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

    /// <summary>A validated UI protocol or execution failure.</summary>
    public sealed class BattlementUiException : InvalidOperationException
    {
        public BattlementUiException(CoreErrorCode errorCode, string message)
            : base(message) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
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
