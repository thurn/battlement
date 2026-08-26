#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityClickEvent = UnityEngine.UIElements.ClickEvent;
using UnityNavigationSubmitEvent = UnityEngine.UIElements.NavigationSubmitEvent;
using UnityTransitionCancelEvent = UnityEngine.UIElements.TransitionCancelEvent;
using UnityTransitionEndEvent = UnityEngine.UIElements.TransitionEndEvent;
using UnityTransitionStartEvent = UnityEngine.UIElements.TransitionStartEvent;

namespace Battlement.UI
{
    /// <summary>Constructs and populates Battlement-owned UI Toolkit documents.</summary>
    public sealed class BattlementUiDocuments
    {
        private const int MaximumHierarchyDepth = 256;

        private readonly Dictionary<Guid, UnityEngine.UIElements.VisualElement> elements = new();
        private readonly Dictionary<UnityEngine.UIElements.VisualElement, Guid> elementIds = new();
        private readonly Dictionary<Guid, Guid> documentRoots = new();
        private readonly Dictionary<Guid, Guid?> parentIds = new();
        private readonly Dictionary<Guid, List<Guid>> logicalChildren = new();
        private readonly HashSet<Guid> rootIds = new();
        private readonly Dictionary<Guid, System.Action> repeatActions = new();
        private readonly Dictionary<Guid, (long Delay, long Interval)> repeatTimings = new();
        private readonly Dictionary<Guid, (long Delay, long Interval)> pendingRepeatTimings = new();
        private readonly HashSet<Guid> pressedRepeatButtons = new();
        private readonly BattlementUiElementProperties properties;
        private readonly BattlementUiEventForwarder events;
        private readonly BattlementUiScrollControls scrollControls;
        private readonly BattlementUiTabControls tabControls;
        private readonly BattlementUiTextFieldControls textFieldControls;
        private readonly Func<Guid, bool>? isWorldObject;
        private readonly Action<IReadOnlyList<Guid>>? reserveIdentities;
        private readonly Action<IReadOnlyList<Guid>>? releaseIdentities;

        /// <summary>Creates a document manager with an optional synchronous event sink.</summary>
        public BattlementUiDocuments(
            Func<UiEvent, bool>? emitUiEvent = null,
            Func<Guid, bool>? containsWorldObject = null,
            Action<IReadOnlyList<Guid>>? reserveUiIdentities = null,
            Action<IReadOnlyList<Guid>>? releaseUiIdentities = null,
            IBattlementUiAssetLookup? assetLookup = null,
            Func<TimeSpan>? now = null
        )
        {
            properties = new BattlementUiElementProperties(emitUiEvent, assetLookup);
            events = properties.EventForwarder;
            scrollControls = new BattlementUiScrollControls(
                properties.EventForwarder,
                now ?? (() => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble))
            );
            tabControls = new BattlementUiTabControls(properties.EventForwarder);
            textFieldControls = new BattlementUiTextFieldControls(properties.EventForwarder);
            isWorldObject = containsWorldObject;
            reserveIdentities = reserveUiIdentities;
            releaseIdentities = releaseUiIdentities;
        }

        /// <summary>Creates an empty native UI-document GameObject.</summary>
        public static GameObject CreateGameObject(GameObjectKind.UiDocumentState description) =>
            BattlementUiDocumentFactory.Create(description);

        /// <summary>Replaces tracked hierarchies from an authoritative snapshot.</summary>
        public void Replace(
            IReadOnlyList<UiDocument>? descriptions,
            Func<ObjectId, GameObject?> resolveGameObject
        )
        {
            elements.Clear();
            elementIds.Clear();
            properties.Clear();
            scrollControls.Clear();
            tabControls.Clear();
            textFieldControls.Clear();
            documentRoots.Clear();
            parentIds.Clear();
            logicalChildren.Clear();
            rootIds.Clear();
            repeatActions.Clear();
            repeatTimings.Clear();
            pendingRepeatTimings.Clear();
            pressedRepeatButtons.Clear();
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
                properties.ApplyRoot(root, description.RootId, description);
                Reserve(description.RootId, root, description.RootId.Value);
                rootIds.Add(description.RootId.Value);
                RegisterRootNavigation(root);
                foreach (UiNode child in description.Children ?? Array.Empty<UiNode>())
                {
                    UnityEngine.UIElements.VisualElement created = CreateElement(
                        child,
                        description.RootId.Value,
                        description.RootId.Value
                    );
                    tabControls.Insert(root, created);
                    logicalChildren[description.RootId.Value].Add(child.ObjectId.Value);
                }
            }
        }

        /// <summary>Finds a tracked document root or authored element.</summary>
        public bool TryGet(ObjectId objectId, out UnityEngine.UIElements.VisualElement? value) =>
            elements.TryGetValue(objectId.Value, out value);

        /// <summary>Gets the identities currently owned by UI Toolkit elements.</summary>
        public IEnumerable<Guid> IdentityIds => elements.Keys;

        /// <summary>Advances coalesced live scroll events and settlement deadlines.</summary>
        public void Advance()
        {
            scrollControls.Advance();
            textFieldControls.Advance();
        }

        /// <summary>Releases every tracked root and element identity.</summary>
        public void Clear()
        {
            releaseIdentities?.Invoke(new List<Guid>(elements.Keys));
            elements.Clear();
            elementIds.Clear();
            properties.Clear();
            scrollControls.Clear();
            tabControls.Clear();
            textFieldControls.Clear();
            documentRoots.Clear();
            parentIds.Clear();
            logicalChildren.Clear();
            rootIds.Clear();
            repeatActions.Clear();
            repeatTimings.Clear();
            pendingRepeatTimings.Clear();
            pressedRepeatButtons.Clear();
        }

        /// <summary>Creates and attaches one validated element subtree.</summary>
        public void Create(CommandBody.VisualElement.Create command)
        {
            UnityEngine.UIElements.VisualElement parent = Require(command.ParentId);
            RequireContainer(parent, command.ParentId);
            ValidatePlacement(command.Node.Element, parent);
            int index = command.ChildIndex is uint requested
                ? checked((int)requested)
                : logicalChildren[command.ParentId.Value].Count;
            if (index > logicalChildren[command.ParentId.Value].Count)
            {
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            }

            var ids = new HashSet<Guid>();
            ValidateDetached(command.Node, ids, 0);
            int parentDepth = DepthOf(command.ParentId.Value);
            if (parentDepth + SubtreeDepth(command.Node) + 1 > MaximumHierarchyDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            var reserved = new List<Guid>(ids);
            reserveIdentities?.Invoke(reserved);
            try
            {
                Guid rootId = documentRoots[command.ParentId.Value];
                UnityEngine.UIElements.VisualElement created = CreateElement(
                    command.Node,
                    rootId,
                    command.ParentId.Value
                );
                tabControls.Insert(parent, created, command.ChildIndex is null ? null : index);
                logicalChildren[command.ParentId.Value].Insert(index, command.Node.ObjectId.Value);
            }
            catch
            {
                foreach (Guid id in ids)
                {
                    RemoveIdentity(id);
                }
                releaseIdentities?.Invoke(reserved);
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
                    this.properties.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    scrollControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    tabControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    textFieldControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    if (properties.Element is UiElement.RepeatButton repeat)
                        ApplyRepeatTiming(
                            (UnityEngine.UIElements.RepeatButton)target,
                            properties.ObjectId,
                            repeat
                        );
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
            List<Guid> removed = SubtreeIds(command.ObjectId.Value);
            tabControls.Remove(target);
            Guid parentId =
                parentIds[command.ObjectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            logicalChildren[parentId].Remove(command.ObjectId.Value);
            foreach (Guid id in removed)
                RemoveIdentity(id);
            releaseIdentities?.Invoke(removed);
        }

        /// <summary>Performs one supported transient native UI operation.</summary>
        public void PerformAction(CommandBody.VisualElement.PerformAction command)
        {
            if (command.Action is VisualElementAction.ScrollTo scrollTo)
            {
                UnityEngine.UIElements.VisualElement target = Require(command.ObjectId);
                if (target is not UnityEngine.UIElements.ScrollView scroll)
                    throw Failure(
                        CoreErrorCode.ComponentMissing,
                        "ScrollTo requires a ScrollView."
                    );
                UnityEngine.UIElements.VisualElement descendant = Require(scrollTo.DescendantId);
                if (!IsDescendant(scrollTo.DescendantId.Value, command.ObjectId.Value))
                    throw Failure(
                        CoreErrorCode.InvalidHierarchy,
                        "ScrollTo requires a logical descendant of its target."
                    );
                scrollControls.ScrollTo(command.ObjectId, scroll, descendant);
                return;
            }
            throw Failure(
                CoreErrorCode.InvalidProperty,
                $"UI action {command.Action.GetType().Name} is unsupported by this executor."
            );
        }

        private UnityEngine.UIElements.VisualElement CreateElement(
            UiNode node,
            Guid documentRoot,
            Guid parentId
        )
        {
            UiElement description = node.Element;
            UnityEngine.UIElements.VisualElement value = description switch
            {
                UiElement.VisualElement => new UnityEngine.UIElements.VisualElement(),
                UiElement.Box => new UnityEngine.UIElements.Box(),
                UiElement.Label label => new UnityEngine.UIElements.Label(
                    label.Text ?? string.Empty
                ),
                UiElement.TextElement text => new UnityEngine.UIElements.TextElement
                {
                    text = text.Text ?? string.Empty,
                },
                UiElement.TextField text => new UnityEngine.UIElements.TextField(
                    text.Label ?? string.Empty
                ),
                UiElement.Button button => new UnityEngine.UIElements.Button
                {
                    text = button.Text ?? string.Empty,
                },
                UiElement.RepeatButton repeat => CreateRepeatButton(node.ObjectId, repeat),
                UiElement.GroupBox group => new UnityEngine.UIElements.GroupBox(
                    group.Text ?? string.Empty
                ),
                UiElement.PopupWindow popup => new UnityEngine.UIElements.PopupWindow
                {
                    text = popup.Text ?? string.Empty,
                },
                UiElement.ScrollView => new UnityEngine.UIElements.ScrollView(),
                UiElement.Scroller => new UnityEngine.UIElements.Scroller(),
                UiElement.Tab tab => new UnityEngine.UIElements.Tab(tab.Text ?? string.Empty),
                UiElement.TabView => new UnityEngine.UIElements.TabView(),
                UiElement.Image => new UnityEngine.UIElements.Image(),
                _ => throw new InvalidOperationException("Unsupported UI element type."),
            };

            Populate(value, node, documentRoot, parentId);
            if (description is UiElement.Button)
                value.RegisterCallback<UnityClickEvent>(eventValue =>
                    events.ForwardClick(node.ObjectId, eventValue)
                );
            value.RegisterCallback<UnityTransitionStartEvent>(eventValue =>
                events.ForwardTransition(
                    node.ObjectId,
                    UiEventKind.TransitionStart,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            value.RegisterCallback<UnityTransitionEndEvent>(eventValue =>
                events.ForwardTransition(
                    node.ObjectId,
                    UiEventKind.TransitionEnd,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            value.RegisterCallback<UnityTransitionCancelEvent>(eventValue =>
                events.ForwardTransition(
                    node.ObjectId,
                    UiEventKind.TransitionCancel,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            return value;
        }

        private UnityEngine.UIElements.RepeatButton CreateRepeatButton(
            ObjectId objectId,
            UiElement.RepeatButton value
        )
        {
            if (value.DelayMs is not uint delay || value.IntervalMs is not uint interval)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "RepeatButton creation requires delay and interval."
                );
            if (interval == 0)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "RepeatButton interval must be positive."
                );
            System.Action callback = () => events.ForwardRepeat(Route(objectId.Value));
            repeatActions.Add(objectId.Value, callback);
            repeatTimings.Add(objectId.Value, (delay, interval));
            var result = new UnityEngine.UIElements.RepeatButton(callback, delay, interval)
            {
                text = value.Text ?? string.Empty,
            };
            result.RegisterCallback<PointerDownEvent>(_ =>
                pressedRepeatButtons.Add(objectId.Value)
            );
            result.RegisterCallback<PointerUpEvent>(_ =>
                ReleaseRepeatButton(result, objectId.Value)
            );
            return result;
        }

        private void ApplyRepeatTiming(
            UnityEngine.UIElements.RepeatButton target,
            ObjectId objectId,
            UiElement.RepeatButton value
        )
        {
            if (value.DelayMs is null && value.IntervalMs is null)
                return;
            (long delay, long interval) = pendingRepeatTimings.TryGetValue(
                objectId.Value,
                out (long Delay, long Interval) pending
            )
                ? pending
                : repeatTimings[objectId.Value];
            delay = value.DelayMs is uint nextDelay ? nextDelay : delay;
            interval = value.IntervalMs is uint nextInterval ? nextInterval : interval;
            if (interval <= 0)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "RepeatButton interval must be positive."
                );
            if (pressedRepeatButtons.Contains(objectId.Value))
            {
                pendingRepeatTimings[objectId.Value] = (delay, interval);
                return;
            }
            target.SetAction(repeatActions[objectId.Value], delay, interval);
            repeatTimings[objectId.Value] = (delay, interval);
        }

        private void ReleaseRepeatButton(UnityEngine.UIElements.RepeatButton target, Guid objectId)
        {
            pressedRepeatButtons.Remove(objectId);
            if (!pendingRepeatTimings.Remove(objectId, out (long Delay, long Interval) timing))
                return;
            long previousInterval = repeatTimings[objectId].Interval;
            target
                .schedule.Execute(() =>
                {
                    target.SetAction(repeatActions[objectId], timing.Delay, timing.Interval);
                    repeatTimings[objectId] = timing;
                })
                .StartingIn(previousInterval + 1);
        }

        private void RegisterRootNavigation(UnityEngine.UIElements.VisualElement root) =>
            root.RegisterCallback<UnityNavigationSubmitEvent>(eventValue =>
            {
                Guid? targetId = NearestId(
                    eventValue.target as UnityEngine.UIElements.VisualElement
                );
                if (targetId is not Guid id)
                    return;
                bool isButton =
                    elements[id] is UnityEngine.UIElements.Button
                    && elements[id] is not UnityEngine.UIElements.RepeatButton;
                events.ForwardNavigationSubmit(Route(id), isButton);
            });

        private Guid? NearestId(UnityEngine.UIElements.VisualElement? target)
        {
            for (
                UnityEngine.UIElements.VisualElement? value = target;
                value is not null;
                value = value.parent
            )
            {
                if (elementIds.TryGetValue(value, out Guid objectId))
                    return objectId;
            }
            return null;
        }

        private IReadOnlyList<Guid> Route(Guid objectId)
        {
            var result = new List<Guid>();
            Guid? current = objectId;
            while (current is Guid value)
            {
                result.Add(value);
                current = parentIds[value];
            }
            return result;
        }

        private void Populate(
            UnityEngine.UIElements.VisualElement value,
            UiNode node,
            Guid documentRoot,
            Guid parentId
        )
        {
            properties.ApplyElement(value, node.ObjectId, node.Element);
            Reserve(node.ObjectId, value, documentRoot, parentId);
            scrollControls.ApplyCreate(value, node.ObjectId, node.Element);
            tabControls.ApplyCreate(value, node.ObjectId, node.Element);
            textFieldControls.ApplyCreate(value, node.ObjectId, node.Element);
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
            {
                tabControls.Insert(value, CreateElement(child, documentRoot, node.ObjectId.Value));
                logicalChildren[node.ObjectId.Value].Add(child.ObjectId.Value);
            }
            if (node.Element is UiElement.TabView tabView)
                tabControls.Initialize(
                    (UnityEngine.UIElements.TabView)value,
                    node.ObjectId,
                    tabView.SelectedTabIndex
                );
        }

        private void Reserve(
            ObjectId objectId,
            UnityEngine.UIElements.VisualElement value,
            Guid documentRoot,
            Guid? parentId = null
        )
        {
            if (!elements.TryAdd(objectId.Value, value))
            {
                throw new InvalidOperationException($"UI identity {objectId} is duplicated.");
            }
            elementIds.Add(value, objectId.Value);
            documentRoots.Add(objectId.Value, documentRoot);
            parentIds.Add(objectId.Value, parentId);
            logicalChildren.Add(objectId.Value, new List<Guid>());
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
                if (isWorldObject?.Invoke(objectId.Value) == true)
                    throw Failure(
                        CoreErrorCode.ComponentMissing,
                        $"Object {objectId} is not a UI element."
                    );
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
            if (
                value
                is UnityEngine.UIElements.Label
                    or UnityEngine.UIElements.Button
                    or UnityEngine.UIElements.RepeatButton
                    or UnityEngine.UIElements.Image
            )
            {
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    $"UI element {objectId} cannot contain children."
                );
            }
        }

        private void ValidateDetached(UiNode node, ISet<Guid> ids, int depth)
        {
            if (node.ObjectId.Value == Guid.Empty)
                throw Failure(CoreErrorCode.InvalidProperty, "UI identities must be nonzero.");
            if (!ids.Add(node.ObjectId.Value) || elements.ContainsKey(node.ObjectId.Value))
                throw Failure(
                    CoreErrorCode.DuplicateId,
                    $"UI identity {node.ObjectId} is duplicated."
                );
            if (elements.Count + ids.Count > 100_000)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI identity limit was exceeded.");
            if (depth > MaximumHierarchyDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            BattlementUiElementProperties.Validate(node.Element, allowUsageHints: true);
            IReadOnlyList<UiNode> children = node.Children ?? Array.Empty<UiNode>();
            if (
                node.Element
                    is UiElement.Label
                        or UiElement.TextElement
                        or UiElement.RepeatButton
                        or UiElement.Button
                        or UiElement.Scroller
                        or UiElement.TextField
                        or UiElement.Image
                && children.Count != 0
            )
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Leaf UI controls cannot contain logical children."
                );
            if (
                node.Element is UiElement.TabView tabView
                && tabView.SelectedTabIndex is uint selected
                && selected >= children.Count
            )
                throw Failure(CoreErrorCode.InvalidProperty, "Selected tab index is out of range.");
            foreach (UiNode child in children)
            {
                ValidatePlacement(child.Element, node.Element);
                ValidateDetached(child, ids, depth + 1);
            }
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
                UiElement.TextElement => target.GetType()
                    == typeof(UnityEngine.UIElements.TextElement),
                UiElement.TextField => target.GetType() == typeof(UnityEngine.UIElements.TextField),
                UiElement.Button => target.GetType() == typeof(UnityEngine.UIElements.Button),
                UiElement.RepeatButton => target.GetType()
                    == typeof(UnityEngine.UIElements.RepeatButton),
                UiElement.GroupBox => target.GetType() == typeof(UnityEngine.UIElements.GroupBox),
                UiElement.PopupWindow => target.GetType()
                    == typeof(UnityEngine.UIElements.PopupWindow),
                UiElement.ScrollView => target.GetType()
                    == typeof(UnityEngine.UIElements.ScrollView),
                UiElement.Scroller => target.GetType() == typeof(UnityEngine.UIElements.Scroller),
                UiElement.Tab => target.GetType() == typeof(UnityEngine.UIElements.Tab),
                UiElement.TabView => target.GetType() == typeof(UnityEngine.UIElements.TabView),
                UiElement.Image => target.GetType() == typeof(UnityEngine.UIElements.Image),
                _ => false,
            };
            if (!matches)
                throw new InvalidOperationException(
                    $"UI element {objectId} update has the wrong concrete class."
                );
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
            ValidatePlacement(target, parent);
            if (documentRoots[objectId.Value] != documentRoots[parentId.Value])
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "UI elements cannot move between documents."
                );
            if (target == parent || IsDescendant(parentId.Value, objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A UI placement cannot create a cycle."
                );
            if (DepthOf(parentId.Value) + SubtreeDepth(objectId.Value) + 1 > MaximumHierarchyDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            Guid oldParent =
                parentIds[objectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            tabControls.Remove(target);
            tabControls.Insert(parent, target);
            logicalChildren[oldParent].Remove(objectId.Value);
            logicalChildren[parentId.Value].Add(objectId.Value);
            parentIds[objectId.Value] = parentId.Value;
        }

        private void ApplyIndex(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            uint childIndex
        )
        {
            if (rootIds.Contains(objectId.Value))
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be reordered."
                );
            Guid parentId =
                parentIds[objectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            UnityEngine.UIElements.VisualElement parent = elements[parentId];
            int index = checked((int)childIndex);
            if (index >= logicalChildren[parentId].Count)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            int previousIndex = logicalChildren[parentId].IndexOf(objectId.Value);
            if (parent is UnityEngine.UIElements.TabView tabView)
                tabControls.Reorder(tabView, previousIndex, index);
            else
            {
                tabControls.Remove(target);
                tabControls.Insert(parent, target, index);
            }
            logicalChildren[parentId].Remove(objectId.Value);
            logicalChildren[parentId].Insert(index, objectId.Value);
        }

        private int DepthOf(Guid objectId)
        {
            int depth = 0;
            Guid? cursor = objectId;
            while (cursor is Guid value && parentIds[value] is Guid parent)
            {
                depth++;
                cursor = parent;
            }
            return depth;
        }

        private int SubtreeDepth(Guid objectId)
        {
            int depth = 0;
            foreach (Guid child in logicalChildren[objectId])
                depth = Math.Max(depth, SubtreeDepth(child) + 1);
            return depth;
        }

        private static int SubtreeDepth(UiNode node)
        {
            int depth = 0;
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                depth = Math.Max(depth, SubtreeDepth(child) + 1);
            return depth;
        }

        private bool IsDescendant(Guid candidate, Guid ancestor)
        {
            Guid? cursor = candidate;
            while (cursor is Guid value)
            {
                if (value == ancestor)
                    return true;
                cursor = parentIds[value];
            }
            return false;
        }

        private List<Guid> SubtreeIds(Guid objectId)
        {
            var result = new List<Guid> { objectId };
            foreach (Guid child in logicalChildren[objectId])
                result.AddRange(SubtreeIds(child));
            result.Reverse();
            return result;
        }

        private static void ValidatePlacement(
            UiElement child,
            UnityEngine.UIElements.VisualElement parent
        ) => ValidateTabPlacement(child is UiElement.Tab, parent is UnityEngine.UIElements.TabView);

        private static void ValidatePlacement(UiElement child, UiElement parent) =>
            ValidateTabPlacement(child is UiElement.Tab, parent is UiElement.TabView);

        private static void ValidatePlacement(
            UnityEngine.UIElements.VisualElement child,
            UnityEngine.UIElements.VisualElement parent
        ) =>
            ValidateTabPlacement(
                child is UnityEngine.UIElements.Tab,
                parent is UnityEngine.UIElements.TabView
            );

        private static void ValidateTabPlacement(bool childIsTab, bool parentIsTabView)
        {
            if (childIsTab != parentIsTabView)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Tabs must be direct TabView children, and TabViews accept only Tabs."
                );
        }

        private void RemoveIdentity(Guid objectId)
        {
            if (elements.Remove(objectId, out UnityEngine.UIElements.VisualElement value))
            {
                elementIds.Remove(value);
                tabControls.RemoveIdentity(objectId, value);
                textFieldControls.Remove(objectId);
            }
            properties.Remove(objectId);
            scrollControls.Remove(objectId);
            repeatActions.Remove(objectId);
            repeatTimings.Remove(objectId);
            pendingRepeatTimings.Remove(objectId);
            pressedRepeatButtons.Remove(objectId);
            documentRoots.Remove(objectId);
            parentIds.Remove(objectId);
            logicalChildren.Remove(objectId);
        }

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }

    /// <summary>A validated UI protocol or execution failure.</summary>
    public sealed class BattlementUiException : InvalidOperationException
    {
        public BattlementUiException(CoreErrorCode errorCode, string message)
            : base(message) => ErrorCode = errorCode;

        public CoreErrorCode ErrorCode { get; }
    }
}
