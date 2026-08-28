#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
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
        private readonly Dictionary<Guid, UIDocument> rootDocuments = new();
        private readonly Dictionary<Guid, Guid?> parentIds = new();
        private readonly Dictionary<Guid, List<Guid>> logicalChildren = new();
        private readonly HashSet<Guid> rootIds = new();
        private readonly BattlementUiElementProperties properties;
        private readonly BattlementUiEventForwarder events;
        private readonly BattlementUiEventObserver eventObserver;
        private readonly BattlementUiLifecycleEvents lifecycleEvents;
        private readonly BattlementUiScrollControls scrollControls;
        private readonly BattlementUiActions actions;
        private readonly BattlementUiTabControls tabControls;
        private readonly BattlementUiTextFieldControls textFieldControls;
        private readonly BattlementUiBooleanControls booleanControls;
        private readonly BattlementUiChoiceControls choiceControls;
        private readonly BattlementUiDropdownControls dropdownControls;
        private readonly BattlementUiSliderControls sliderControls;
        private readonly BattlementUiRangeControls rangeControls;
        private readonly BattlementUiPartProperties partProperties;
        private readonly BattlementUiRepeatControls repeatControls;
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
            eventObserver = new BattlementUiEventObserver(
                events,
                NearestId,
                Route,
                id =>
                    elements[id] is UnityEngine.UIElements.Button
                    && elements[id] is not UnityEngine.UIElements.RepeatButton
            );
            lifecycleEvents = new BattlementUiLifecycleEvents(events, Route);
            scrollControls = new BattlementUiScrollControls(
                properties.EventForwarder,
                now ?? (() => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble))
            );
            actions = new BattlementUiActions(Require, IsDescendant, scrollControls);
            tabControls = new BattlementUiTabControls(properties.EventForwarder);
            textFieldControls = new BattlementUiTextFieldControls(properties.EventForwarder);
            booleanControls = new BattlementUiBooleanControls(properties.EventForwarder);
            choiceControls = new BattlementUiChoiceControls(properties.EventForwarder);
            dropdownControls = new BattlementUiDropdownControls(properties.EventForwarder);
            sliderControls = new BattlementUiSliderControls(properties.EventForwarder);
            rangeControls = new BattlementUiRangeControls(properties.EventForwarder);
            partProperties = new BattlementUiPartProperties(assetLookup);
            repeatControls = new BattlementUiRepeatControls(events, Route);
            isWorldObject = containsWorldObject;
            reserveIdentities = reserveUiIdentities;
            releaseIdentities = releaseUiIdentities;
        }

        /// <summary>Creates an empty native UI-document GameObject.</summary>
        public static GameObject CreateGameObject(
            GameObjectKind.UiDocumentState description,
            IBattlementUiAssetLookup? assetLookup = null
        ) => BattlementUiDocumentFactory.Create(description, assetLookup);

        /// <summary>Replaces tracked hierarchies from an authoritative snapshot.</summary>
        public void Replace(
            IReadOnlyList<UiDocument>? descriptions,
            Func<ObjectId, GameObject?> resolveGameObject
        )
        {
            eventObserver.Clear();
            lifecycleEvents.Clear();
            elements.Clear();
            elementIds.Clear();
            properties.Clear();
            scrollControls.Clear();
            tabControls.Clear();
            textFieldControls.Clear();
            booleanControls.Clear();
            choiceControls.Clear();
            dropdownControls.Clear();
            sliderControls.Clear();
            rangeControls.Clear();
            partProperties.Clear();
            documentRoots.Clear();
            rootDocuments.Clear();
            parentIds.Clear();
            logicalChildren.Clear();
            rootIds.Clear();
            repeatControls.Clear();
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
                rootDocuments.Add(description.RootId.Value, document);
                rootIds.Add(description.RootId.Value);
                eventObserver.RegisterRoot(root);
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
            lifecycleEvents.SetInputEnabled(true);
        }

        /// <summary>Finds a tracked document root or authored element.</summary>
        public bool TryGet(ObjectId objectId, out UnityEngine.UIElements.VisualElement? value) =>
            elements.TryGetValue(objectId.Value, out value);

        internal bool TryGetGeometryTarget(
            ObjectId objectId,
            out UnityEngine.UIElements.VisualElement element,
            out ObjectId panelId,
            out UIDocument document
        )
        {
            if (
                !elements.TryGetValue(objectId.Value, out element)
                || !documentRoots.TryGetValue(objectId.Value, out Guid rootId)
                || !rootDocuments.TryGetValue(rootId, out document)
            )
            {
                panelId = default;
                element = null!;
                document = null!;
                return false;
            }

            panelId = new ObjectId(rootId);
            return true;
        }

        /// <summary>Gets the identities currently owned by UI Toolkit elements.</summary>
        public IEnumerable<Guid> IdentityIds => elements.Keys;

        internal int LinkIdentityCount => lifecycleEvents.LinkIdentityCount;

        /// <summary>Advances coalesced live scroll events and settlement deadlines.</summary>
        public void Advance()
        {
            lifecycleEvents.Advance();
            scrollControls.Advance();
            textFieldControls.Advance();
            sliderControls.Advance();
            rangeControls.Advance();
        }

        /// <summary>Clears transient interaction state when user input is disabled.</summary>
        public void SetInputEnabled(bool enabled)
        {
            events.SetInputEnabled(enabled);
            lifecycleEvents.SetInputEnabled(enabled);
            if (enabled)
                return;
            eventObserver.Clear();
            textFieldControls.CancelAll();
            scrollControls.CancelAll();
            sliderControls.CancelAll();
            rangeControls.CancelAll();
            repeatControls.CancelAll();
            actions.CancelAll(elements);
        }

        /// <summary>Releases every tracked root and element identity.</summary>
        public void Clear()
        {
            eventObserver.Clear();
            lifecycleEvents.Clear();
            releaseIdentities?.Invoke(new List<Guid>(elements.Keys));
            elements.Clear();
            elementIds.Clear();
            properties.Clear();
            scrollControls.Clear();
            tabControls.Clear();
            textFieldControls.Clear();
            booleanControls.Clear();
            choiceControls.Clear();
            dropdownControls.Clear();
            sliderControls.Clear();
            rangeControls.Clear();
            partProperties.Clear();
            documentRoots.Clear();
            rootDocuments.Clear();
            parentIds.Clear();
            logicalChildren.Clear();
            rootIds.Clear();
            repeatControls.Clear();
        }

        /// <summary>Creates and attaches one validated element subtree.</summary>
        public void Create(CommandBody.VisualElement.Create command)
        {
            UnityEngine.UIElements.VisualElement parent = Require(command.ParentId);
            RequireContainer(parent, command.ParentId);
            ValidatePlacement(command.Node.Element, parent);
            if (
                parent is UnityEngine.UIElements.ToggleButtonGroup
                && logicalChildren[command.ParentId.Value].Count >= 64
            )
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
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
                choiceControls.BeginHierarchyMutation(command.ParentId);
                tabControls.Insert(parent, created, command.ChildIndex is null ? null : index);
                logicalChildren[command.ParentId.Value].Insert(index, command.Node.ObjectId.Value);
                choiceControls.Insert(
                    command.ParentId,
                    index,
                    logicalChildren[command.ParentId.Value].Count
                );
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
                {
                    UnityEngine.UIElements.VisualElement target = Require(properties.ObjectId);
                    RequireElementKind(target, properties.Element, properties.ObjectId);
                    BattlementUiElementProperties.Validate(
                        properties.Element,
                        allowUsageHints: false
                    );
                    BattlementUiChoiceControls.ValidateUpdate(
                        properties.Element,
                        target,
                        logicalChildren[properties.ObjectId.Value].Count
                    );
                    BattlementUiDropdownControls.ValidateUpdate(properties.Element, target);
                    BattlementUiSliderControls.ValidateUpdate(properties.Element, target);
                    BattlementUiRangeControls.ValidateUpdate(properties.Element, target);
                    BattlementUiScrollControls.ValidateUpdate(target, properties.Element);
                    BattlementUiTabControls.ValidateUpdate(target, properties.Element);
                    textFieldControls.ValidateUpdate(properties.ObjectId, properties.Element);
                    using BattlementUiPartProperties.PreparedUpdate preparedParts =
                        partProperties.Prepare(target, properties.ObjectId, properties.Element);
                    this.properties.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    scrollControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    tabControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    textFieldControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    booleanControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    choiceControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    dropdownControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    sliderControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    rangeControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    preparedParts.Commit(properties.ObjectId.Value);
                    if (properties.Element is UiElement.RepeatButton repeat)
                        repeatControls.ApplyUpdate(
                            (UnityEngine.UIElements.RepeatButton)target,
                            properties.ObjectId,
                            repeat
                        );
                    break;
                }
                case VisualElementUpdate.Parent parent:
                    ApplyParent(
                        Require(parent.ObjectId),
                        parent.ObjectId,
                        parent.ParentId,
                        parent.ChildIndex
                    );
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
            Guid parentId =
                parentIds[command.ObjectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            int removedIndex = logicalChildren[parentId].IndexOf(command.ObjectId.Value);
            choiceControls.BeginHierarchyMutation(new ObjectId(parentId));
            tabControls.Remove(target);
            logicalChildren[parentId].Remove(command.ObjectId.Value);
            choiceControls.Remove(
                new ObjectId(parentId),
                removedIndex,
                logicalChildren[parentId].Count
            );
            foreach (Guid id in removed)
                RemoveIdentity(id);
            releaseIdentities?.Invoke(removed);
            eventObserver.Clear();
        }

        /// <summary>Performs one supported transient native UI operation.</summary>
        public void PerformAction(CommandBody.VisualElement.PerformAction command) =>
            actions.Perform(command);

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
                UiElement.Label => new UnityEngine.UIElements.Label(),
                UiElement.TextElement => new UnityEngine.UIElements.TextElement(),
                UiElement.TextField text => new UnityEngine.UIElements.TextField(
                    text.Label.IsSet ? text.Label.Value : string.Empty
                ),
                UiElement.Toggle toggle => new UnityEngine.UIElements.Toggle(
                    toggle.Label.IsSet ? toggle.Label.Value : string.Empty
                ),
                UiElement.RadioButton radio => new UnityEngine.UIElements.RadioButton(
                    radio.Label.IsSet ? radio.Label.Value : string.Empty
                ),
                UiElement.RadioButtonGroup radio => new UnityEngine.UIElements.RadioButtonGroup(
                    radio.Label.IsSet ? radio.Label.Value : string.Empty,
                    new List<string>(
                        radio.Choices.IsSet ? radio.Choices.Value : Array.Empty<string>()
                    )
                ),
                UiElement.ToggleButtonGroup toggle => CreateToggleButtonGroup(node, toggle),
                UiElement.DropdownField dropdown => new UnityEngine.UIElements.DropdownField(
                    dropdown.Label.IsSet ? dropdown.Label.Value : string.Empty,
                    new List<string>(
                        dropdown.Choices.IsSet ? dropdown.Choices.Value : Array.Empty<string>()
                    ),
                    dropdown.Selection.IsSet && dropdown.Selection.Value.Index is uint selected
                        ? checked((int)selected)
                        : -1
                ),
                UiElement.Button => new UnityEngine.UIElements.Button(),
                UiElement.RepeatButton repeat => repeatControls.Create(node.ObjectId, repeat),
                UiElement.GroupBox => new UnityEngine.UIElements.GroupBox(),
                UiElement.PopupWindow => new UnityEngine.UIElements.PopupWindow(),
                UiElement.ScrollView => new UnityEngine.UIElements.ScrollView(),
                UiElement.Scroller => new UnityEngine.UIElements.Scroller(),
                UiElement.Slider => new UnityEngine.UIElements.Slider(),
                UiElement.SliderInt => new UnityEngine.UIElements.SliderInt(),
                UiElement.MinMaxSlider => new UnityEngine.UIElements.MinMaxSlider(),
                UiElement.ProgressBar => new UnityEngine.UIElements.ProgressBar(),
                UiElement.Tab => new UnityEngine.UIElements.Tab(),
                UiElement.TabView => new UnityEngine.UIElements.TabView(),
                UiElement.Image => new UnityEngine.UIElements.Image(),
                _ => throw new InvalidOperationException("Unsupported UI element type."),
            };

            Populate(value, node, documentRoot, parentId);
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

        private static UnityEngine.UIElements.ToggleButtonGroup CreateToggleButtonGroup(
            UiNode node,
            UiElement.ToggleButtonGroup value
        )
        {
            int childCount = (node.Children ?? Array.Empty<UiNode>()).Count;
            bool allowEmpty = value.AllowEmptySelection.IsSet && value.AllowEmptySelection.Value;
            IReadOnlyList<uint> selected =
                value.SelectedIndices.IsSet ? value.SelectedIndices.Value
                : childCount == 0 || allowEmpty ? Array.Empty<uint>()
                : new uint[] { 0 };
            ulong mask = 0;
            foreach (uint index in selected)
                mask |= 1UL << checked((int)index);
            return new UnityEngine.UIElements.ToggleButtonGroup(
                value.Label.IsSet ? value.Label.Value : string.Empty,
                new ToggleButtonGroupState(mask, childCount)
            );
        }

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
            if (!parentIds.ContainsKey(objectId))
                return Array.Empty<Guid>();
            var result = new List<Guid>();
            Guid? current = objectId;
            while (current is Guid value)
            {
                result.Add(value);
                current = parentIds.TryGetValue(value, out Guid? parent) ? parent : null;
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
            booleanControls.ApplyCreate(value, node.ObjectId, node.Element);
            choiceControls.ApplyCreate(value, node.ObjectId, node.Element);
            dropdownControls.ApplyCreate(value, node.ObjectId, node.Element);
            sliderControls.ApplyCreate(value, node.ObjectId, node.Element);
            rangeControls.ApplyCreate(value, node.ObjectId, node.Element);
            partProperties.Apply(value, node.ObjectId, node.Element);
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
            if (node.Element is UiElement.ToggleButtonGroup)
                choiceControls.InitializeToggle(
                    node.ObjectId,
                    logicalChildren[node.ObjectId.Value].Count
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
            eventObserver.RegisterElement(objectId, value);
            lifecycleEvents.Register(objectId, value);
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
                    or UnityEngine.UIElements.Toggle
                    or UnityEngine.UIElements.RadioButton
                    or UnityEngine.UIElements.RadioButtonGroup
                    or UnityEngine.UIElements.DropdownField
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
                        or UiElement.Toggle
                        or UiElement.RadioButton
                        or UiElement.RadioButtonGroup
                        or UiElement.DropdownField
                        or UiElement.Slider
                        or UiElement.SliderInt
                        or UiElement.MinMaxSlider
                        or UiElement.ProgressBar
                        or UiElement.Image
                && children.Count != 0
            )
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Leaf UI controls cannot contain logical children."
                );
            if (
                node.Element is UiElement.TabView tabView
                && tabView.SelectedTabIndex.IsSet
                && tabView.SelectedTabIndex.Value >= children.Count
            )
                throw Failure(CoreErrorCode.InvalidProperty, "Selected tab index is out of range.");
            BattlementUiChoiceControls.ValidateNode(node.Element, children.Count);
            BattlementUiDropdownControls.ValidateNode(node.Element);
            BattlementUiSliderControls.ValidateNode(node.Element);
            BattlementUiRangeControls.ValidateNode(node.Element);
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
                UiElement.Toggle => target.GetType() == typeof(UnityEngine.UIElements.Toggle),
                UiElement.RadioButton => target.GetType()
                    == typeof(UnityEngine.UIElements.RadioButton),
                UiElement.RadioButtonGroup => target.GetType()
                    == typeof(UnityEngine.UIElements.RadioButtonGroup),
                UiElement.ToggleButtonGroup => target.GetType()
                    == typeof(UnityEngine.UIElements.ToggleButtonGroup),
                UiElement.DropdownField => target.GetType()
                    == typeof(UnityEngine.UIElements.DropdownField),
                UiElement.Slider => target.GetType() == typeof(UnityEngine.UIElements.Slider),
                UiElement.SliderInt => target.GetType() == typeof(UnityEngine.UIElements.SliderInt),
                UiElement.MinMaxSlider => target.GetType()
                    == typeof(UnityEngine.UIElements.MinMaxSlider),
                UiElement.ProgressBar => target.GetType()
                    == typeof(UnityEngine.UIElements.ProgressBar),
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
            ObjectId parentId,
            uint? childIndex
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
            Guid oldParent =
                parentIds[objectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            if (
                oldParent != parentId.Value
                && parent is UnityEngine.UIElements.ToggleButtonGroup
                && logicalChildren[parentId.Value].Count >= 64
            )
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
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
            int oldIndex = logicalChildren[oldParent].IndexOf(objectId.Value);
            int destinationLength =
                logicalChildren[parentId.Value].Count - (oldParent == parentId.Value ? 1 : 0);
            int newIndex = childIndex is null ? destinationLength : checked((int)childIndex.Value);
            if (newIndex > destinationLength)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            choiceControls.BeginHierarchyMutation(new ObjectId(oldParent));
            choiceControls.BeginHierarchyMutation(parentId);
            tabControls.Remove(target);
            tabControls.Insert(parent, target, newIndex);
            logicalChildren[oldParent].Remove(objectId.Value);
            logicalChildren[parentId.Value].Insert(newIndex, objectId.Value);
            parentIds[objectId.Value] = parentId.Value;
            if (oldParent == parentId.Value)
                choiceControls.Reorder(parentId, oldIndex, newIndex);
            else
            {
                choiceControls.Remove(
                    new ObjectId(oldParent),
                    oldIndex,
                    logicalChildren[oldParent].Count
                );
                choiceControls.Insert(parentId, newIndex, logicalChildren[parentId.Value].Count);
            }
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
            choiceControls.BeginHierarchyMutation(new ObjectId(parentId));
            if (parent is UnityEngine.UIElements.TabView tabView)
                tabControls.Reorder(tabView, previousIndex, index);
            else
            {
                tabControls.Remove(target);
                tabControls.Insert(parent, target, index);
            }
            logicalChildren[parentId].Remove(objectId.Value);
            logicalChildren[parentId].Insert(index, objectId.Value);
            choiceControls.Reorder(new ObjectId(parentId), previousIndex, index);
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
        ) =>
            ValidatePlacement(
                child is UiElement.Tab,
                parent is UnityEngine.UIElements.TabView,
                child is UiElement.Button,
                parent is UnityEngine.UIElements.ToggleButtonGroup
            );

        private static void ValidatePlacement(UiElement child, UiElement parent) =>
            ValidatePlacement(
                child is UiElement.Tab,
                parent is UiElement.TabView,
                child is UiElement.Button,
                parent is UiElement.ToggleButtonGroup
            );

        private static void ValidatePlacement(
            UnityEngine.UIElements.VisualElement child,
            UnityEngine.UIElements.VisualElement parent
        ) =>
            ValidatePlacement(
                child is UnityEngine.UIElements.Tab,
                parent is UnityEngine.UIElements.TabView,
                child is UnityEngine.UIElements.Button,
                parent is UnityEngine.UIElements.ToggleButtonGroup
            );

        private static void ValidatePlacement(
            bool childIsTab,
            bool parentIsTabView,
            bool childIsButton,
            bool parentIsToggleGroup
        )
        {
            if (childIsTab != parentIsTabView)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Tabs must be direct TabView children, and TabViews accept only Tabs."
                );
            if (parentIsToggleGroup && !childIsButton)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "ToggleButtonGroup accepts only direct Button children."
                );
        }

        private void RemoveIdentity(Guid objectId)
        {
            if (elements.Remove(objectId, out UnityEngine.UIElements.VisualElement value))
            {
                actions.Remove(new ObjectId(objectId), value);
                elementIds.Remove(value);
                lifecycleEvents.Remove(objectId);
                tabControls.RemoveIdentity(objectId, value);
                textFieldControls.Remove(objectId);
                booleanControls.Remove(objectId);
                choiceControls.Remove(objectId);
                dropdownControls.Remove(objectId);
                sliderControls.Remove(objectId);
                rangeControls.Remove(objectId);
                partProperties.Remove(objectId);
            }
            properties.Remove(objectId);
            scrollControls.Remove(objectId);
            repeatControls.Remove(objectId);
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
