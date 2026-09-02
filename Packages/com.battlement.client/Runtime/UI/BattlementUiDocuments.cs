#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using UnityTransitionCancelEvent = UnityEngine.UIElements.TransitionCancelEvent;
using UnityTransitionEndEvent = UnityEngine.UIElements.TransitionEndEvent;
using UnityTransitionStartEvent = UnityEngine.UIElements.TransitionStartEvent;

namespace Battlement.UI
{
    /// <summary>Constructs and populates Battlement-owned UI Toolkit documents.</summary>
    public sealed class BattlementUiDocuments : IDisposable
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
        private readonly BattlementStickyCoordinator stickyCoordinator = new();
        private readonly BattlementOverlayCoordinator overlayCoordinator;
        private readonly BattlementFocusCoordinator focusCoordinator;
        private readonly BattlementAccessibilityManager accessibility;
        private readonly BattlementPresentationLayout presentationLayout;
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
        private readonly BattlementMotionWorld motionWorld;
        private readonly Func<Guid, bool>? isWorldObject;
        private readonly Action<IReadOnlyList<Guid>>? reserveIdentities;
        private readonly Action<IReadOnlyList<Guid>>? releaseIdentities;

        /// <summary>Creates a document manager with an optional synchronous event sink.</summary>
        public BattlementUiDocuments(
            Func<UiEvent, UiEventDisposition?>? emitUiEvent = null,
            Func<Guid, bool>? containsWorldObject = null,
            Action<IReadOnlyList<Guid>>? reserveUiIdentities = null,
            Action<IReadOnlyList<Guid>>? releaseUiIdentities = null,
            IBattlementUiAssetLookup? assetLookup = null,
            Func<TimeSpan>? now = null,
            Func<ObjectId, (TimeSpan Elapsed, bool Discontinuity)>? audioTime = null,
            System.Action? uiEventPreventionApplied = null
        )
        {
            properties = new BattlementUiElementProperties(
                emitUiEvent,
                assetLookup,
                uiEventPreventionApplied
            );
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
            focusCoordinator = new BattlementFocusCoordinator(
                () => elements.Values,
                IsOverlayScopeMember,
                OverlayScopeTraversal,
                id => elements.TryGetValue(id, out VisualElement value) ? value : null
            );
            overlayCoordinator = new BattlementOverlayCoordinator(
                id => elements.TryGetValue(id.Value, out VisualElement value) ? value : null,
                SourceOrdinal,
                IsOverlayScopeMember,
                PhysicalOverlayScopeTraversal,
                focusCoordinator.RefreshModalBoundary
            );
            focusCoordinator.SetModalResolver(overlayCoordinator.ActiveModal);
            events.SetInertPredicate(focusCoordinator.IsEffectivelyInert);
            accessibility = new BattlementAccessibilityManager(
                emitUiEvent,
                id => elements.TryGetValue(id, out VisualElement value) ? value : null,
                element => elementIds.TryGetValue(element, out Guid id) ? id : null,
                focusCoordinator.IsEffectivelyInert,
                focusCoordinator.ActiveModal
            );
            presentationLayout = new BattlementPresentationLayout(
                stickyCoordinator,
                overlayCoordinator
            );
            motionWorld = new BattlementMotionWorld(
                assetLookup: assetLookup,
                audioTime: audioTime is null
                    ? null
                    : id =>
                    {
                        (TimeSpan elapsed, bool discontinuity) = audioTime(id);
                        return new MotionClockSample(
                            checked((ulong)(elapsed.TotalMilliseconds * 1000)),
                            discontinuity
                        );
                    },
                resolveElement: id =>
                    elements.TryGetValue(id.Value, out VisualElement value) ? value : null,
                gestureTime: now,
                presentationChanged: presentationLayout.Refresh
            );
            focusCoordinator.SetFocusVisibleWriter(
                (target, visible) =>
                {
                    if (elementIds.TryGetValue(target, out Guid id))
                        motionWorld.SetFocusVisible(new ObjectId(id), visible);
                }
            );
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
            Func<ObjectId, GameObject?> resolveGameObject,
            bool preserveMotion = false
        )
        {
            (UiDocument Description, UIDocument Document)[] resolved =
                BattlementDocumentReconstruction.Resolve(
                    descriptions ?? Array.Empty<UiDocument>(),
                    resolveGameObject
                );
            VisualElement[] previousRoots = rootIds.Select(value => elements[value]).ToArray();
            if (preserveMotion)
                motionWorld.BeginReconnect();
            else
                motionWorld.Clear();
            try
            {
                eventObserver.Clear();
                lifecycleEvents.Clear();
                stickyCoordinator.Clear();
                overlayCoordinator.Clear();
                focusCoordinator.Clear();
                accessibility.Clear(reconnect: preserveMotion);
                foreach (VisualElement root in previousRoots)
                    root.Clear();
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
                foreach ((UiDocument description, UIDocument document) in resolved)
                {
                    UnityEngine.UIElements.VisualElement root = document.rootVisualElement;
                    root.Clear();
                    properties.ApplyRoot(root, description.RootId, description);
                    Reserve(description.RootId, root, description.RootId.Value);
                    focusCoordinator.ApplyRoot(root, description);
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
                        ApplyStickySubtree(created);
                        ApplyOverlaySubtree(created);
                    }
                }
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
                accessibility.Refresh();
                lifecycleEvents.SetInputEnabled(true);
                if (preserveMotion)
                    motionWorld.EndReconnect();
            }
            catch
            {
                if (preserveMotion)
                    motionWorld.AbortReconnect();
                else
                    motionWorld.Clear();
                throw;
            }
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

        internal IEnumerable<UIDocument> InputDocuments => rootDocuments.Values;

        internal BattlementMotionWorld MotionWorldForTests => motionWorld;

        internal BattlementAccessibilityManager AccessibilityForTests => accessibility;

        internal IReadOnlyCollection<AccessibilityNodeSnapshot> ActiveAccessibility =>
            accessibility.Active;

        internal bool DispatchAccessibility(ObjectId target, AccessibilityAction action) =>
            accessibility.Dispatch(
                new AccessibilityEvent(accessibility.Generation, target, action)
            );

        /// <summary>Returns diagnostics for the most recently presented Motion frame.</summary>
        public BattlementMotionPerformanceSnapshot MotionPerformance => motionWorld.Performance;

        internal MotionEventBatch? TakeMotionEvents() => motionWorld.DrainEventBatch();

        internal void RecordMotionTraffic(int payloadBytes) =>
            motionWorld.RecordPerformanceTraffic(payloadBytes);

        internal void Apply(MotionValueOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionValuePlaybackOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionPlaybackOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionControlledClockOperation operation) =>
            motionWorld.Apply(operation);

        internal void Apply(MotionControlOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionScopeOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionDragControlOperation operation) => motionWorld.Apply(operation);

        internal bool TryFindNearestId(
            UnityEngine.UIElements.VisualElement? element,
            out ObjectId objectId
        )
        {
            Guid? nearest = NearestId(element);
            objectId = nearest is Guid id ? new ObjectId(id) : default;
            return nearest is not null;
        }

        /// <summary>Gets the identities currently owned by UI Toolkit elements.</summary>
        public IEnumerable<Guid> IdentityIds => elements.Keys;

        internal int LinkIdentityCount => lifecycleEvents.LinkIdentityCount;

        internal ulong DittoLayoutFingerprint()
        {
            const ulong offset = 14_695_981_039_346_656_037;
            const ulong prime = 1_099_511_628_211;
            ulong hash = offset;
            foreach (
                KeyValuePair<Guid, VisualElement> entry in elements.OrderBy(value => value.Key)
            )
            {
                foreach (byte value in entry.Key.ToByteArray())
                {
                    hash = (hash ^ value) * prime;
                }

                UnityEngine.Rect layout = entry.Value.layout;
                hash = (hash ^ (uint)BitConverter.SingleToInt32Bits(layout.x)) * prime;
                hash = (hash ^ (uint)BitConverter.SingleToInt32Bits(layout.y)) * prime;
                hash = (hash ^ (uint)BitConverter.SingleToInt32Bits(layout.width)) * prime;
                hash = (hash ^ (uint)BitConverter.SingleToInt32Bits(layout.height)) * prime;
            }
            return hash;
        }

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
            focusCoordinator.SetInputEnabled(enabled);
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

        internal void BeginCommit()
        {
            accessibility.Suspend();
            focusCoordinator.BeginCommit();
        }

        internal void EndCommit()
        {
            focusCoordinator.EndCommit();
            accessibility.Resume();
        }

        internal void Apply(AccessibilityUpdatePayload update) => accessibility.Apply(update);

        /// <summary>Releases every tracked root and element identity.</summary>
        public void Clear()
        {
            motionWorld.Clear();
            eventObserver.Clear();
            lifecycleEvents.Clear();
            stickyCoordinator.Clear();
            overlayCoordinator.Clear();
            focusCoordinator.Clear();
            accessibility.Clear();
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

        /// <summary>Releases motion loop integration and tracked UI state.</summary>
        public void Dispose()
        {
            Clear();
            accessibility.Dispose();
            motionWorld.Dispose();
        }

        /// <summary>Creates and attaches one validated element subtree.</summary>
        public void Create(CommandBody.VisualElement.Create command)
        {
            UnityEngine.UIElements.VisualElement parent = Require(command.ParentId);
            RequireContainer(parent, command.ParentId);
            ValidatePlacement(command.Node.Element, parent);
            ValidateOverlayContexts(
                command.Node,
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
            );
            ValidateStickySubtree(command.Node, HasScrollAncestor(parent));
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
            ValidateOverlayPlacement(
                command.Node.ObjectId,
                command.Node.Element,
                parent,
                command.Node
            );
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
                InsertNativeChild(parent, created, command.ChildIndex is null ? null : index);
                logicalChildren[command.ParentId.Value].Insert(index, command.Node.ObjectId.Value);
                ApplyStickySubtree(created);
                ApplyOverlaySubtree(created);
                RefreshStickyOrdinals();
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
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
                    bool genericRootUpdate =
                        rootIds.Contains(properties.ObjectId.Value)
                        && properties.Element is UiElement.VisualElement;
                    if (!genericRootUpdate)
                        RequireElementKind(target, properties.Element, properties.ObjectId);
                    ValidateLayoutUpdate(target, properties.Element);
                    ValidateStickyUpdate(target, properties.ObjectId, properties.Element);
                    ValidateOverlayUpdate(target, properties.ObjectId, properties.Element);
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
                    BattlementPreparedMotionAdmission? preparedMotion = motionWorld.Prepare(
                        target,
                        properties.ObjectId,
                        properties.Element.Motion
                    );
                    this.properties.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    focusCoordinator.ApplyUpdate(target, properties.Element);
                    BattlementGridItems.Apply(target, properties.Element.GridItem);
                    BattlementStackItems.Apply(target, properties.Element.StackItem);
                    BattlementStickyItems.Apply(target, properties.Element.Sticky);
                    if (
                        target is BattlementLayoutContainer layout
                        && properties.Element is UiElement.Flex flex
                    )
                        layout.ApplyFlex(flex);
                    if (
                        target is BattlementLayoutContainer gridLayout
                        && properties.Element is UiElement.Grid grid
                    )
                        gridLayout.ApplyGrid(grid);
                    if (
                        target is BattlementLayoutContainer stackLayout
                        && properties.Element is UiElement.Stack stack
                    )
                        stackLayout.ApplyStack(stack);
                    RefreshParentLayout(properties.ObjectId.Value);
                    stickyCoordinator.Apply(
                        target,
                        properties.Element.Sticky,
                        SourceOrdinal(target)
                    );
                    overlayCoordinator.Apply(target, properties.Element.OverlayPlacement);
                    scrollControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    tabControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    textFieldControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    booleanControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    choiceControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    dropdownControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    sliderControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    rangeControls.ApplyUpdate(target, properties.ObjectId, properties.Element);
                    preparedParts.Commit(properties.ObjectId.Value);
                    preparedMotion?.Commit();
                    if (properties.Element is UiElement.RepeatButton repeat)
                        repeatControls.ApplyUpdate(
                            (UnityEngine.UIElements.RepeatButton)target,
                            properties.ObjectId,
                            repeat
                        );
                    overlayCoordinator.RefreshAll();
                    focusCoordinator.Refresh();
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
            stickyCoordinator.PrepareHierarchyChange(target);
            RemoveNativeChild(elements[parentId], target);
            logicalChildren[parentId].Remove(command.ObjectId.Value);
            choiceControls.Remove(
                new ObjectId(parentId),
                removedIndex,
                logicalChildren[parentId].Count
            );
            foreach (Guid id in removed)
                RemoveIdentity(id);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
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
                UiElement.Flex => new BattlementLayoutContainer(BattlementLayoutContainerKind.Flex),
                UiElement.Grid => new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid),
                UiElement.Stack => new BattlementLayoutContainer(
                    BattlementLayoutContainerKind.Stack
                ),
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
            BattlementPreparedMotionAdmission? preparedMotion = motionWorld.Prepare(
                value,
                node.ObjectId,
                node.Element.Motion
            );
            properties.ApplyElement(value, node.ObjectId, node.Element);
            focusCoordinator.ApplyCreate(value, node.Element);
            BattlementGridItems.Apply(value, node.Element.GridItem);
            BattlementStackItems.Apply(value, node.Element.StackItem);
            BattlementStickyItems.Apply(value, node.Element.Sticky);
            BattlementOverlayItems.Apply(value, node.Element.OverlayPlacement);
            if (value is BattlementLayoutContainer layout && node.Element is UiElement.Flex flex)
                layout.ApplyFlex(flex);
            if (
                value is BattlementLayoutContainer gridLayout
                && node.Element is UiElement.Grid grid
            )
                gridLayout.ApplyGrid(grid);
            if (
                value is BattlementLayoutContainer stackLayout
                && node.Element is UiElement.Stack stack
            )
                stackLayout.ApplyStack(stack);
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
            preparedMotion?.Commit();
            BattlementLayoutContainer? updatingLayout = value as BattlementLayoutContainer;
            updatingLayout?.BeginUpdate();
            try
            {
                foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                {
                    InsertNativeChild(
                        value,
                        CreateElement(child, documentRoot, node.ObjectId.Value),
                        null
                    );
                    logicalChildren[node.ObjectId.Value].Add(child.ObjectId.Value);
                }
            }
            finally
            {
                updatingLayout?.EndUpdate();
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
                UiElement.Flex => target is BattlementLayoutContainer layout
                    && layout.Kind == BattlementLayoutContainerKind.Flex,
                UiElement.Grid => target is BattlementLayoutContainer grid
                    && grid.Kind == BattlementLayoutContainerKind.Grid,
                UiElement.Stack => target is BattlementLayoutContainer stack
                    && stack.Kind == BattlementLayoutContainerKind.Stack,
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
            if (BattlementOverlayItems.HasAuthored(target))
                overlayCoordinator.Validate(
                    objectId,
                    BattlementOverlayItems.Get(target),
                    parent,
                    IsDescendant
                );
            if (BattlementStickyItems.HasAuthored(target) && !HasScrollAncestor(parent))
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
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
            stickyCoordinator.PrepareHierarchyChange(target);
            overlayCoordinator.PrepareHierarchyChange(target);
            focusCoordinator.PrepareHierarchyChange(target);
            RemoveNativeChild(elements[oldParent], target);
            InsertNativeChild(parent, target, newIndex);
            logicalChildren[oldParent].Remove(objectId.Value);
            logicalChildren[parentId.Value].Insert(newIndex, objectId.Value);
            parentIds[objectId.Value] = parentId.Value;
            ApplyStickyAfterAttachment(target);
            ApplyOverlayAfterAttachment(target);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
            focusCoordinator.CompleteHierarchyChange();
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
            stickyCoordinator.PrepareHierarchyChange(target);
            overlayCoordinator.PrepareHierarchyChange(target);
            if (parent is BattlementLayoutContainer layout)
                layout.Adapter.Reindex(target, index);
            else if (parent is UnityEngine.UIElements.TabView tabView)
                tabControls.Reorder(tabView, previousIndex, index);
            else
            {
                tabControls.Remove(target);
                tabControls.Insert(parent, target, index);
            }
            logicalChildren[parentId].Remove(objectId.Value);
            logicalChildren[parentId].Insert(index, objectId.Value);
            ApplyStickyAfterAttachment(target);
            ApplyOverlayAfterAttachment(target);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
            choiceControls.Reorder(new ObjectId(parentId), previousIndex, index);
        }

        private void InsertNativeChild(
            UnityEngine.UIElements.VisualElement parent,
            UnityEngine.UIElements.VisualElement child,
            int? index
        )
        {
            if (parent is BattlementLayoutContainer layout)
            {
                layout.Adapter.Insert(child, index ?? layout.Adapter.Count);
                return;
            }
            tabControls.Insert(parent, child, index);
        }

        private void RemoveNativeChild(
            UnityEngine.UIElements.VisualElement parent,
            UnityEngine.UIElements.VisualElement child
        )
        {
            if (parent is BattlementLayoutContainer layout)
            {
                layout.Adapter.Detach(child);
                return;
            }
            tabControls.Remove(child);
        }

        private void RefreshParentLayout(Guid objectId)
        {
            if (
                parentIds.TryGetValue(objectId, out Guid? parentId)
                && parentId is Guid value
                && elements[value] is BattlementLayoutContainer layout
            )
            {
                layout.FlexLayout?.Refresh();
                layout.GridLayout?.Invalidate();
                layout.StackLayout?.Invalidate();
            }
        }

        private void ApplyStickyAfterAttachment(UnityEngine.UIElements.VisualElement target)
        {
            if (!BattlementStickyItems.HasAuthored(target))
                return;
            stickyCoordinator.Apply(
                target,
                Prop<Sticky>.Set(BattlementStickyItems.Get(target)),
                SourceOrdinal(target)
            );
        }

        private void ApplyStickySubtree(UnityEngine.UIElements.VisualElement target)
        {
            ApplyStickyAfterAttachment(target);
            if (!elementIds.TryGetValue(target, out Guid id))
                return;
            foreach (Guid child in logicalChildren[id])
                ApplyStickySubtree(elements[child]);
        }

        private void RefreshStickyOrdinals() => stickyCoordinator.RefreshOrdinals(SourceOrdinal);

        private void ApplyOverlayAfterAttachment(UnityEngine.UIElements.VisualElement target)
        {
            if (!BattlementOverlayItems.HasAuthored(target))
                return;
            Guid id = elementIds[target];
            overlayCoordinator.Validate(
                new ObjectId(id),
                BattlementOverlayItems.Get(target),
                target.hierarchy.parent
                    ?? throw Failure(
                        CoreErrorCode.InvalidHierarchy,
                        "Overlay wrapper is not attached."
                    ),
                IsDescendant
            );
            overlayCoordinator.Apply(
                target,
                Prop<OverlayPlacement>.Set(BattlementOverlayItems.Get(target))
            );
        }

        private void ApplyOverlaySubtree(UnityEngine.UIElements.VisualElement target)
        {
            ApplyOverlayAfterAttachment(target);
            if (!elementIds.TryGetValue(target, out Guid id))
                return;
            foreach (Guid child in logicalChildren[id])
                ApplyOverlaySubtree(elements[child]);
        }

        private void RefreshOverlayOrdinals() => overlayCoordinator.RefreshOrdinals();

        private void ValidateOverlayPlacement(
            ObjectId objectId,
            UiElement element,
            UnityEngine.UIElements.VisualElement parent,
            UiNode? pendingTree = null
        )
        {
            if (!element.OverlayPlacement.IsSet)
                return;
            ValidateOverlayHost(parent);
            overlayCoordinator.Validate(
                objectId,
                element.OverlayPlacement.Value,
                parent,
                (candidate, ancestor) =>
                    pendingTree is not null && ContainsDetached(pendingTree, ancestor)
                        ? IsDetachedDescendant(pendingTree, candidate, ancestor)
                        : IsDescendant(candidate, ancestor),
                id => pendingTree is not null && ContainsDetached(pendingTree, id.Value)
            );
        }

        private static bool ContainsDetached(UiNode node, Guid candidate)
        {
            if (node.ObjectId.Value == candidate)
                return true;
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
            {
                if (ContainsDetached(child, candidate))
                    return true;
            }
            return false;
        }

        private static bool IsDetachedDescendant(UiNode node, Guid candidate, Guid ancestor)
        {
            if (node.ObjectId.Value == ancestor)
            {
                foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                {
                    if (ContainsDetached(child, candidate))
                        return true;
                }
                return false;
            }
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
            {
                if (IsDetachedDescendant(child, candidate, ancestor))
                    return true;
            }
            return false;
        }

        private void ValidateOverlayUpdate(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement element
        )
        {
            OverlayPlacement? current = BattlementOverlayItems.HasAuthored(target)
                ? BattlementOverlayItems.Get(target)
                : null;
            bool modal = element.OverlayPlacement.IsSet
                ? element.OverlayPlacement.Value is OverlayPlacement.Modal
                : element.OverlayPlacement.IsUnset && current is OverlayPlacement.Modal;
            if (modal)
                ValidateModalFocusProperties(element);
            if (!element.OverlayPlacement.IsSet)
            {
                if (element.OverlayPlacement.IsUnset && BattlementOverlayItems.HasAuthored(target))
                    BattlementUiElementValidator.ValidateOverlayStyle(
                        element.Style,
                        BattlementOverlayItems.Get(target)
                    );
                return;
            }
            UnityEngine.UIElements.VisualElement parent =
                target.hierarchy.parent
                ?? throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Overlay wrapper is not attached."
                );
            ValidateOverlayHost(parent);
            overlayCoordinator.Validate(
                objectId,
                element.OverlayPlacement.Value,
                parent,
                IsDescendant
            );
        }

        private static void ValidateModalFocusProperties(UiElement element)
        {
            if (element.Enabled.IsSet && !element.Enabled.Value)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper must be enabled.");
            if (element.Focusable.IsSet && !element.Focusable.Value)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper must be focusable.");
            if (element.TabIndex.IsSet && element.TabIndex.Value != -1)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "A modal wrapper must use tab index -1."
                );
            if (element.Inert.IsSet && element.Inert.Value)
                throw Failure(CoreErrorCode.InvalidProperty, "A modal wrapper cannot be inert.");
        }

        private static void ValidateOverlayContexts(UiNode node, bool parentIsStack)
        {
            if (node.Element.OverlayPlacement.IsSet && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Overlay placement requires a direct OverlayHost Stack target."
                );
            bool nodeIsStack = node.Element is UiElement.Stack;
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                ValidateOverlayContexts(child, nodeIsStack);
        }

        private void ValidateOverlayHost(UnityEngine.UIElements.VisualElement physicalParent)
        {
            BattlementLayoutContainer host = physicalParent switch
            {
                BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack } direct =>
                    direct,
                BattlementLayoutSlot
                {
                    ContainingBlock: BattlementLayoutContainer
                    {
                        Kind: BattlementLayoutContainerKind.Stack
                    } slotted
                } => slotted,
                _ => throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Overlay placement requires a direct OverlayHost Stack target."
                ),
            };
            if (!elementIds.TryGetValue(host, out Guid hostId))
                throw Failure(CoreErrorCode.InvalidHierarchy, "OverlayHost is not registered.");
            Guid rootStackId =
                parentIds[hostId]
                ?? throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "OverlayHost requires a document-root Stack."
                );
            bool finalChild = logicalChildren[rootStackId].LastOrDefault() == hostId;
            bool rootStack =
                elements[rootStackId]
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
                && parentIds[rootStackId] is Guid documentRoot
                && rootIds.Contains(documentRoot);
            StackItem item = BattlementStackItems.Get(host);
            bool configured =
                BattlementStackItems.HasAuthored(host)
                && item.Order == int.MaxValue
                && !item.ContributesToSize
                && host.pickingMode == PickingMode.Ignore
                && host.style.overflow.value == Overflow.Visible;
            if (!rootStack || !finalChild || !configured)
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "OverlayHost must be the configured final child of a document-root Stack."
                );
        }

        private int SourceOrdinal(UnityEngine.UIElements.VisualElement target)
        {
            if (!elementIds.TryGetValue(target, out Guid targetId))
                return int.MaxValue;
            Guid root = documentRoots[targetId];
            int ordinal = 0;
            return FindOrdinal(root, targetId, ref ordinal) ? ordinal : int.MaxValue;
        }

        private bool IsOverlayScopeMember(
            UnityEngine.UIElements.VisualElement candidate,
            UnityEngine.UIElements.VisualElement scope
        ) =>
            candidate.panel == scope.panel
            && elementIds.TryGetValue(candidate, out Guid candidateId)
            && elementIds.TryGetValue(scope, out Guid scopeId)
            && IsDescendant(candidateId, scopeId);

        private IEnumerable<UnityEngine.UIElements.VisualElement> OverlayScopeTraversal(
            UnityEngine.UIElements.VisualElement scope
        )
        {
            if (!elementIds.TryGetValue(scope, out Guid scopeId))
                yield break;
            foreach (Guid id in LogicalPreorder(scopeId))
                yield return elements[id];
        }

        private IEnumerable<UnityEngine.UIElements.VisualElement> PhysicalOverlayScopeTraversal(
            UnityEngine.UIElements.VisualElement scope
        )
        {
            if (!elementIds.TryGetValue(scope, out Guid scopeId) || scope.panel is null)
                yield break;
            foreach (
                UnityEngine.UIElements.VisualElement candidate in PhysicalPreorder(
                    scope.panel.visualTree
                )
            )
            {
                if (
                    elementIds.TryGetValue(candidate, out Guid candidateId)
                    && IsDescendant(candidateId, scopeId)
                )
                    yield return candidate;
            }
        }

        private static IEnumerable<UnityEngine.UIElements.VisualElement> PhysicalPreorder(
            UnityEngine.UIElements.VisualElement parent
        )
        {
            yield return parent;
            foreach (UnityEngine.UIElements.VisualElement child in parent.Children())
            {
                foreach (UnityEngine.UIElements.VisualElement descendant in PhysicalPreorder(child))
                    yield return descendant;
            }
        }

        private IEnumerable<Guid> LogicalPreorder(Guid objectId)
        {
            yield return objectId;
            foreach (Guid child in logicalChildren[objectId])
            {
                foreach (Guid descendant in LogicalPreorder(child))
                    yield return descendant;
            }
        }

        private bool FindOrdinal(Guid current, Guid target, ref int ordinal)
        {
            foreach (Guid child in logicalChildren[current])
            {
                if (child == target)
                    return true;
                ordinal++;
                if (FindOrdinal(child, target, ref ordinal))
                    return true;
            }
            return false;
        }

        private void ValidateStickyUpdate(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            UiElement value
        )
        {
            bool remainsSticky =
                value.Sticky.IsSet
                || (value.Sticky.IsUnset && BattlementStickyItems.HasAuthored(target));
            if (!remainsSticky)
                return;
            Guid parentId =
                parentIds[objectId.Value]
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            if (!HasScrollAncestor(elements[parentId]))
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
            bool absolute =
                value.Style?.Position.IsSet == true
                    ? value.Style.Position.Value.Keyword is null
                        && value.Style.Position.Value.Value == UiPosition.Absolute
                    : target.style.position.value == Position.Absolute;
            if (absolute)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires relative positioning."
                );
        }

        private static void ValidateStickySubtree(UiNode node, bool hasScrollAncestor)
        {
            if (node.Element.Sticky.IsSet && !hasScrollAncestor)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
            bool descendantsHaveScroll = hasScrollAncestor || node.Element is UiElement.ScrollView;
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                ValidateStickySubtree(child, descendantsHaveScroll);
        }

        private static bool HasScrollAncestor(UnityEngine.UIElements.VisualElement value)
        {
            for (
                UnityEngine.UIElements.VisualElement? current = value;
                current is not null;
                current = current.parent
            )
            {
                if (current is UnityEngine.UIElements.ScrollView)
                    return true;
            }
            return false;
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
        )
        {
            ValidatePlacement(
                child is UiElement.Tab,
                parent is UnityEngine.UIElements.TabView,
                child is UiElement.Button,
                parent is UnityEngine.UIElements.ToggleButtonGroup
            );
            ValidateGridPlacement(
                child,
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Grid }
            );
            ValidateStackPlacement(
                child,
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
            );
        }

        private static void ValidatePlacement(UiElement child, UiElement parent)
        {
            ValidatePlacement(
                child is UiElement.Tab,
                parent is UiElement.TabView,
                child is UiElement.Button,
                parent is UiElement.ToggleButtonGroup
            );
            ValidateGridPlacement(child, parent is UiElement.Grid);
            ValidateStackPlacement(child, parent is UiElement.Stack);
        }

        private static void ValidatePlacement(
            UnityEngine.UIElements.VisualElement child,
            UnityEngine.UIElements.VisualElement parent
        )
        {
            ValidatePlacement(
                child is UnityEngine.UIElements.Tab,
                parent is UnityEngine.UIElements.TabView,
                child is UnityEngine.UIElements.Button,
                parent is UnityEngine.UIElements.ToggleButtonGroup
            );
            bool parentIsGrid =
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Grid };
            if (BattlementGridItems.HasAuthored(child) && !parentIsGrid)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "GridItem requires a direct Grid placement context."
                );
            if (parentIsGrid)
                ValidateNativeLayoutStyle(child, "Grid");
            bool parentIsStack =
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack };
            if (BattlementStackItems.HasAuthored(child) && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                ValidateNativeLayoutStyle(child, "Stack");
            if (BattlementStickyItems.HasAuthored(child) && !HasScrollAncestor(parent))
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
        }

        private static void ValidateLayoutUpdate(
            UnityEngine.UIElements.VisualElement target,
            UiElement value
        )
        {
            bool parentIsGrid =
                target.parent is BattlementLayoutSlot slot
                && slot.parent
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Grid };
            if (value.GridItem.IsSet && !parentIsGrid)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "GridItem requires a direct Grid placement context."
                );
            if (parentIsGrid)
                ValidateLayoutStyle(value.Style, "Grid");
            bool parentIsStack =
                target.parent is BattlementLayoutSlot stackSlot
                && stackSlot.parent
                    is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack };
            if (value.StackItem.IsSet && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                ValidateLayoutStyle(value.Style, "Stack");
        }

        private static void ValidateGridPlacement(UiElement child, bool parentIsGrid)
        {
            if (child.GridItem.IsSet && !parentIsGrid)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "GridItem requires a direct Grid placement context."
                );
            if (parentIsGrid)
                ValidateLayoutStyle(child.Style, "Grid");
        }

        private static void ValidateStackPlacement(UiElement child, bool parentIsStack)
        {
            if (child.StackItem.IsSet && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                ValidateLayoutStyle(child.Style, "Stack");
        }

        private static void ValidateLayoutStyle(UiStyle? style, string container)
        {
            if (style is null)
                return;
            bool absolute =
                style.Position.IsSet
                && style.Position.Value.Keyword is null
                && style.Position.Value.Value == UiPosition.Absolute;
            bool offsetsAreAutomatic = new[]
            {
                style.Top,
                style.Right,
                style.Bottom,
                style.Left,
            }.All(LayoutOffsetIsAutomatic);
            if (absolute || !offsetsAreAutomatic)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"{container} placement children require relative position "
                        + "and automatic offsets."
                );
        }

        private static bool LayoutOffsetIsAutomatic(Prop<UiStyleValue<UiLengthOrAuto>> value) =>
            !value.IsSet
            || value.Value.Keyword is not null
            || value.Value.Value is UiLengthOrAuto.Auto;

        private static void ValidateNativeLayoutStyle(
            UnityEngine.UIElements.VisualElement child,
            string container
        )
        {
            bool offsetsAreAutomatic = new[]
            {
                child.style.top,
                child.style.right,
                child.style.bottom,
                child.style.left,
            }.All(value => value.keyword == StyleKeyword.Auto);
            if (child.style.position.value == Position.Absolute || !offsetsAreAutomatic)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"{container} placement children require relative position "
                        + "and automatic offsets."
                );
        }

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
                stickyCoordinator.Remove(value);
                overlayCoordinator.Remove(value);
                focusCoordinator.Remove(value);
                if (value is BattlementLayoutContainer layout)
                    layout.Adapter.Clear();
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
                motionWorld.RemoveHost(new ObjectId(objectId));
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
