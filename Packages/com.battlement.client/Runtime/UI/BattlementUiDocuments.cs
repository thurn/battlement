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
        private readonly BattlementUiHierarchy hierarchy;
        private readonly BattlementUiPlacementValidator placementValidator;
        private readonly BattlementUiElementProperties properties;
        private readonly BattlementUiPropertyUpdates propertyUpdates;
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
        private readonly BattlementUiParticleStreaks particles;
        private readonly BattlementUiTabControls tabControls;
        private readonly BattlementUiTextFieldControls textFieldControls;
        private readonly BattlementUiBooleanControls booleanControls;
        private readonly BattlementUiChoiceControls choiceControls;
        private readonly BattlementUiDropdownControls dropdownControls;
        private readonly BattlementUiSliderControls sliderControls;
        private readonly BattlementUiRangeControls rangeControls;
        private readonly BattlementUiPartProperties partProperties;
        private readonly BattlementUiRepeatControls repeatControls;
        private readonly BattlementUiSyntheticInputAdapter syntheticInput;
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
            System.Action? uiEventPreventionApplied = null,
            Func<TimeSpan>? scaledNow = null,
            Func<bool>? instantMotion = null
        )
        {
            hierarchy = new BattlementUiHierarchy();
            placementValidator = new BattlementUiPlacementValidator(hierarchy);
            Func<TimeSpan> uiTime =
                now ?? (() => TimeSpan.FromSeconds(Time.realtimeSinceStartupAsDouble));
            properties = new BattlementUiElementProperties(
                emitUiEvent,
                assetLookup,
                uiEventPreventionApplied
            );
            events = properties.EventForwarder;
            eventObserver = new BattlementUiEventObserver(
                events,
                hierarchy.NearestId,
                hierarchy.Route,
                id =>
                    hierarchy.TryGet(new ObjectId(id), out VisualElement? element)
                    && element is UnityEngine.UIElements.Button
                    && element is not UnityEngine.UIElements.RepeatButton
            );
            lifecycleEvents = new BattlementUiLifecycleEvents(events, hierarchy.Route);
            scrollControls = new BattlementUiScrollControls(properties.EventForwarder, uiTime);
            particles = new BattlementUiParticleStreaks(uiTime, instantMotion ?? (() => false));
            actions = new BattlementUiActions(
                Require,
                hierarchy.IsDescendant,
                scrollControls,
                particles
            );
            tabControls = new BattlementUiTabControls(properties.EventForwarder);
            textFieldControls = new BattlementUiTextFieldControls(properties.EventForwarder);
            booleanControls = new BattlementUiBooleanControls(properties.EventForwarder);
            syntheticInput = new BattlementUiSyntheticInputAdapter(
                hierarchy,
                events,
                booleanControls
            );
            choiceControls = new BattlementUiChoiceControls(properties.EventForwarder);
            dropdownControls = new BattlementUiDropdownControls(properties.EventForwarder);
            sliderControls = new BattlementUiSliderControls(properties.EventForwarder);
            rangeControls = new BattlementUiRangeControls(properties.EventForwarder);
            partProperties = new BattlementUiPartProperties(assetLookup);
            repeatControls = new BattlementUiRepeatControls(events, hierarchy.Route);
            focusCoordinator = new BattlementFocusCoordinator(
                () => hierarchy.Elements,
                IsOverlayScopeMember,
                OverlayScopeTraversal,
                id => hierarchy.TryGet(new ObjectId(id), out VisualElement? value) ? value : null
            );
            overlayCoordinator = new BattlementOverlayCoordinator(
                id => hierarchy.TryGet(id, out VisualElement? value) ? value : null,
                hierarchy.SourceOrdinal,
                IsOverlayScopeMember,
                PhysicalOverlayScopeTraversal,
                focusCoordinator.RefreshModalBoundary
            );
            focusCoordinator.SetModalResolver(overlayCoordinator.ActiveModal);
            events.SetInertPredicate(focusCoordinator.IsEffectivelyInert);
            accessibility = new BattlementAccessibilityManager(
                emitUiEvent,
                id => hierarchy.TryGet(new ObjectId(id), out VisualElement? value) ? value : null,
                element => hierarchy.TryGetId(element, out Guid id) ? id : null,
                focusCoordinator.IsEffectivelyInert,
                focusCoordinator.ActiveModal
            );
            presentationLayout = new BattlementPresentationLayout(
                stickyCoordinator,
                overlayCoordinator
            );
            motionWorld = new BattlementMotionWorld(
                unscaledTime: () => uiTime().TotalSeconds,
                scaledTime: scaledNow is null ? null : () => scaledNow().TotalSeconds,
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
                resolveElement: id => hierarchy.TryGet(id, out VisualElement? value) ? value : null,
                gestureTime: uiTime,
                presentationChanged: presentationLayout.Refresh
            );
            focusCoordinator.SetFocusVisibleWriter(
                (target, visible) =>
                {
                    if (hierarchy.TryGetId(target, out Guid id))
                        motionWorld.SetFocusVisible(new ObjectId(id), visible);
                }
            );
            propertyUpdates = new BattlementUiPropertyUpdates(
                hierarchy,
                Require,
                placementValidator,
                properties,
                focusCoordinator,
                overlayCoordinator,
                stickyCoordinator,
                motionWorld,
                partProperties,
                scrollControls,
                tabControls,
                textFieldControls,
                booleanControls,
                choiceControls,
                dropdownControls,
                sliderControls,
                rangeControls,
                repeatControls
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
            VisualElement[] previousRoots = hierarchy
                .Roots.Select(value => value.Element)
                .ToArray();
            if (preserveMotion)
                motionWorld.BeginReconnect();
            else
                motionWorld.Clear();
            try
            {
                syntheticInput.Clear();
                particles.Clear();
                eventObserver.Clear();
                lifecycleEvents.Clear();
                stickyCoordinator.Clear();
                overlayCoordinator.Clear();
                focusCoordinator.Clear();
                accessibility.Clear(reconnect: preserveMotion);
                foreach (VisualElement root in previousRoots)
                    root.Clear();
                foreach (VisualElement element in hierarchy.Elements)
                    BattlementPaintProperties.Release(element);
                hierarchy.Clear();
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
                repeatControls.Clear();
                foreach ((UiDocument description, UIDocument document) in resolved)
                {
                    UnityEngine.UIElements.VisualElement root = document.rootVisualElement;
                    root.Clear();
                    properties.CaptureDefaults(root, description.RootId);
                    properties.ApplyRoot(root, description.RootId, description);
                    hierarchy.AddRoot(description.RootId, root, document, RegisterHierarchyEntry);
                    focusCoordinator.ApplyRoot(root, description);
                    eventObserver.RegisterRoot(root);
                    foreach (UiNode child in description.Children ?? Array.Empty<UiNode>())
                    {
                        UnityEngine.UIElements.VisualElement created = CreateElement(
                            child,
                            description.RootId.Value,
                            description.RootId.Value
                        );
                        tabControls.Insert(root, created);
                        hierarchy.AddChild(
                            description.RootId,
                            child.ObjectId,
                            hierarchy.Children(description.RootId.Value).Count
                        );
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
            hierarchy.TryGet(objectId, out value);

        internal bool TryGetGeometryTarget(
            ObjectId objectId,
            out UnityEngine.UIElements.VisualElement element,
            out ObjectId panelId,
            out UIDocument document
        )
        {
            return hierarchy.TryGetGeometryTarget(objectId, out element, out panelId, out document);
        }

        internal IEnumerable<UIDocument> InputDocuments => hierarchy.InputDocuments;

        internal BattlementMotionWorld MotionWorldForTests => motionWorld;

        internal BattlementAccessibilityManager AccessibilityForTests => accessibility;

        internal int DittoActiveFiniteTimelineCount =>
            motionWorld.ActiveFiniteTimelineCount + particles.ActiveCount;

        internal int DittoActiveInfiniteTimelineCount => motionWorld.ActiveInfiniteTimelineCount;

        internal int DittoActiveHeldTimelineCount => motionWorld.ActiveHeldTimelineCount;

        internal string DittoActiveTimelineDiagnostic => motionWorld.ActiveTimelineDiagnostic;

        internal int CompleteDittoPresentedFrame() =>
            repeatControls.CompletePendingSettlement() + motionWorld.CompleteReadySlots();

        internal bool DittoHasPendingDeferredWork =>
            focusCoordinator.HasPendingWork
            || scrollControls.HasPendingSettlement
            || stickyCoordinator.HasPendingWork
            || repeatControls.HasPendingSettlement;

        internal IReadOnlyCollection<AccessibilityNodeSnapshot> ActiveAccessibility =>
            accessibility.Active;

        internal bool DispatchAccessibility(
            ObjectId target,
            AccessibilityAction action,
            out string? diagnostic
        ) =>
            accessibility.Dispatch(
                new AccessibilityEvent(accessibility.Generation, target, action),
                out diagnostic
            );

        internal bool BeginSyntheticPointer(
            ObjectId target,
            Vector2 screenPosition,
            out string? diagnostic
        ) => syntheticInput.BeginSyntheticPointer(target, screenPosition, out diagnostic);

        internal bool DispatchSyntheticHover(
            ObjectId target,
            Vector2 screenPosition,
            out string? diagnostic
        ) => syntheticInput.DispatchSyntheticHover(target, screenPosition, out diagnostic);

        internal bool FinishSyntheticClick(ObjectId target, out string? diagnostic) =>
            syntheticInput.FinishSyntheticClick(target, out diagnostic);

        internal bool DispatchSemanticActivation(ObjectId target, out string? diagnostic) =>
            syntheticInput.DispatchSemanticActivation(target, out diagnostic);

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
            Guid? nearest = hierarchy.NearestId(element);
            objectId = nearest is Guid id ? new ObjectId(id) : default;
            return nearest is not null;
        }

        /// <summary>Gets the identities currently owned by UI Toolkit elements.</summary>
        public IEnumerable<Guid> IdentityIds => hierarchy.IdentityIds;

        internal int LinkIdentityCount => lifecycleEvents.LinkIdentityCount;

        internal ulong DittoLayoutFingerprint()
        {
            const ulong offset = 14_695_981_039_346_656_037;
            const ulong prime = 1_099_511_628_211;
            ulong hash = offset;
            foreach (
                BattlementUiHierarchy.Entry entry in hierarchy.Entries.OrderBy(value => value.Id)
            )
            {
                foreach (byte value in entry.Id.ToByteArray())
                {
                    hash = (hash ^ value) * prime;
                }

                UnityEngine.Rect layout = entry.Element.layout;
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
            particles.Advance();
            foreach (BattlementUiHierarchy.Entry root in hierarchy.Roots)
                BattlementTextSpacing.Refresh(root.Element);
            lifecycleEvents.Advance();
            scrollControls.Advance();
            textFieldControls.Advance();
            sliderControls.Advance();
            rangeControls.Advance();
        }

        /// <summary>Clears transient interaction state when user input is disabled.</summary>
        public void SetInputEnabled(bool enabled)
        {
            syntheticInput.SetInputEnabled(enabled);
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
            actions.CancelAll(hierarchy.Entries);
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
            syntheticInput.Clear();
            particles.Clear();
            motionWorld.Clear();
            eventObserver.Clear();
            lifecycleEvents.Clear();
            stickyCoordinator.Clear();
            overlayCoordinator.Clear();
            focusCoordinator.Clear();
            accessibility.Clear();
            BattlementUiHierarchy.Entry[] tracked = hierarchy.Entries.ToArray();
            releaseIdentities?.Invoke(tracked.Select(value => value.Id).ToArray());
            foreach (BattlementUiHierarchy.Entry entry in tracked)
                BattlementPaintProperties.Release(entry.Element);
            hierarchy.Clear();
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
            ValidateStickySubtree(
                command.Node,
                BattlementUiPlacementValidator.HasScrollAncestor(parent)
            );
            if (
                parent is UnityEngine.UIElements.ToggleButtonGroup
                && hierarchy.Children(command.ParentId.Value).Count >= 64
            )
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
            int index = command.ChildIndex is uint requested
                ? checked((int)requested)
                : hierarchy.Children(command.ParentId.Value).Count;
            if (index > hierarchy.Children(command.ParentId.Value).Count)
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
            int parentDepth = hierarchy.DepthOf(command.ParentId.Value);
            if (parentDepth + SubtreeDepth(command.Node) + 1 > BattlementUiHierarchy.MaximumDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            var reserved = new List<Guid>(ids);
            reserveIdentities?.Invoke(reserved);
            try
            {
                Guid rootId = hierarchy.DocumentRoot(command.ParentId.Value);
                UnityEngine.UIElements.VisualElement created = CreateElement(
                    command.Node,
                    rootId,
                    command.ParentId.Value
                );
                choiceControls.BeginHierarchyMutation(command.ParentId);
                InsertNativeChild(parent, created, command.ChildIndex is null ? null : index);
                hierarchy.AddChild(command.ParentId, command.Node.ObjectId, index);
                ApplyStickySubtree(created);
                ApplyOverlaySubtree(created);
                RefreshStickyOrdinals();
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
                choiceControls.Insert(
                    command.ParentId,
                    index,
                    hierarchy.Children(command.ParentId.Value).Count
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
                    propertyUpdates.Apply(properties);
                    break;
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
            if (hierarchy.IsRoot(command.ObjectId.Value))
            {
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be destroyed by a UI command."
                );
            }
            Guid parentId =
                hierarchy.ParentId(command.ObjectId.Value)
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            int removedIndex = hierarchy.IndexOfChild(parentId, command.ObjectId.Value);
            choiceControls.BeginHierarchyMutation(new ObjectId(parentId));
            stickyCoordinator.PrepareHierarchyChange(target);
            RemoveNativeChild(Require(new ObjectId(parentId)), target);
            IReadOnlyList<BattlementUiHierarchy.Entry> removed = hierarchy.RemoveSubtree(
                command.ObjectId
            );
            choiceControls.Remove(
                new ObjectId(parentId),
                removedIndex,
                hierarchy.Children(parentId).Count
            );
            foreach (BattlementUiHierarchy.Entry entry in removed)
                RemoveIdentity(entry);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
            releaseIdentities?.Invoke(removed.Select(value => value.Id).ToArray());
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
                UiElement.VisualElement => new BattlementPaintHost(),
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

            properties.CaptureDefaults(value, node.ObjectId);
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

        private void Populate(
            UnityEngine.UIElements.VisualElement value,
            UiNode node,
            Guid documentRoot,
            Guid parentId
        )
        {
            using BattlementPreparedMotionAdmission? preparedMotion = motionWorld.Prepare(
                value,
                node.ObjectId,
                node.Element.Motion,
                node.Element.Paint
            );
            properties.ApplyElement(value, node.ObjectId, node.Element);
            BattlementPaintProperties.Apply(value, node.Element.Paint);
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
                    hierarchy.AddChild(
                        new ObjectId(node.ObjectId.Value),
                        child.ObjectId,
                        hierarchy.Children(node.ObjectId.Value).Count
                    );
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
                    hierarchy.Children(node.ObjectId.Value).Count
                );
        }

        private void Reserve(
            ObjectId objectId,
            UnityEngine.UIElements.VisualElement value,
            Guid documentRoot,
            Guid? parentId = null
        )
        {
            hierarchy.Add(objectId, value, documentRoot, parentId, RegisterHierarchyEntry);
        }

        private void RegisterHierarchyEntry(ObjectId objectId, VisualElement element)
        {
            eventObserver.RegisterElement(objectId, element);
            lifecycleEvents.Register(objectId, element);
        }

        private UnityEngine.UIElements.VisualElement Require(ObjectId objectId)
        {
            if (!hierarchy.TryGet(objectId, out UnityEngine.UIElements.VisualElement? value))
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
            return value!;
        }

        private static void RequireContainer(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId
        )
        {
            if (
                value
                is UnityEngine.UIElements.Label
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
            if (!ids.Add(node.ObjectId.Value) || hierarchy.Contains(node.ObjectId.Value))
                throw Failure(
                    CoreErrorCode.DuplicateId,
                    $"UI identity {node.ObjectId} is duplicated."
                );
            if (hierarchy.Count + ids.Count > 100_000)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI identity limit was exceeded.");
            if (depth > BattlementUiHierarchy.MaximumDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            BattlementUiElementProperties.Validate(node.Element, allowUsageHints: true);
            IReadOnlyList<UiNode> children = node.Children ?? Array.Empty<UiNode>();
            if (
                node.Element
                    is UiElement.Label
                        or UiElement.TextElement
                        or UiElement.RepeatButton
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

        private void ApplyParent(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            ObjectId parentId,
            uint? childIndex
        )
        {
            UnityEngine.UIElements.VisualElement parent = Require(parentId);
            RequireContainer(parent, parentId);
            ValidatePlacement(target, parent);
            if (BattlementOverlayItems.HasAuthored(target))
                overlayCoordinator.Validate(
                    objectId,
                    BattlementOverlayItems.Get(target),
                    parent,
                    hierarchy.IsDescendant
                );
            if (
                BattlementStickyItems.HasAuthored(target)
                && !BattlementUiPlacementValidator.HasScrollAncestor(parent)
            )
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
            BattlementUiHierarchy.MovePlan plan = hierarchy.PrepareMove(
                objectId,
                parentId,
                childIndex
            );
            if (
                plan.OldParentId != parentId.Value
                && parent is UnityEngine.UIElements.ToggleButtonGroup
                && hierarchy.Children(parentId.Value).Count >= 64
            )
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
            choiceControls.BeginHierarchyMutation(new ObjectId(plan.OldParentId));
            choiceControls.BeginHierarchyMutation(parentId);
            stickyCoordinator.PrepareHierarchyChange(target);
            overlayCoordinator.PrepareHierarchyChange(target);
            focusCoordinator.PrepareHierarchyChange(target);
            RemoveNativeChild(Require(new ObjectId(plan.OldParentId)), target);
            InsertNativeChild(parent, target, plan.NewIndex);
            hierarchy.ApplyMove(plan);
            ApplyStickyAfterAttachment(target);
            ApplyOverlayAfterAttachment(target);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
            focusCoordinator.CompleteHierarchyChange();
            if (plan.OldParentId == parentId.Value)
                choiceControls.Reorder(parentId, plan.OldIndex, plan.NewIndex);
            else
            {
                choiceControls.Remove(
                    new ObjectId(plan.OldParentId),
                    plan.OldIndex,
                    hierarchy.Children(plan.OldParentId).Count
                );
                choiceControls.Insert(
                    parentId,
                    plan.NewIndex,
                    hierarchy.Children(parentId.Value).Count
                );
            }
        }

        private void ApplyIndex(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId,
            uint childIndex
        )
        {
            BattlementUiHierarchy.ReorderPlan plan = hierarchy.PrepareReorder(objectId, childIndex);
            UnityEngine.UIElements.VisualElement parent = Require(new ObjectId(plan.ParentId));
            particles.Remove(parent);
            choiceControls.BeginHierarchyMutation(new ObjectId(plan.ParentId));
            stickyCoordinator.PrepareHierarchyChange(target);
            overlayCoordinator.PrepareHierarchyChange(target);
            if (parent is BattlementLayoutContainer layout)
                layout.Adapter.Reindex(target, plan.NewIndex);
            else if (parent is UnityEngine.UIElements.TabView tabView)
                tabControls.Reorder(tabView, plan.OldIndex, plan.NewIndex);
            else
            {
                tabControls.Remove(target);
                tabControls.Insert(parent, target, plan.NewIndex);
            }
            hierarchy.ApplyReorder(plan);
            ApplyStickyAfterAttachment(target);
            ApplyOverlayAfterAttachment(target);
            RefreshStickyOrdinals();
            RefreshOverlayOrdinals();
            focusCoordinator.Refresh();
            choiceControls.Reorder(new ObjectId(plan.ParentId), plan.OldIndex, plan.NewIndex);
        }

        private void InsertNativeChild(
            UnityEngine.UIElements.VisualElement parent,
            UnityEngine.UIElements.VisualElement child,
            int? index
        )
        {
            particles.Remove(parent);
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
            particles.Remove(parent);
            if (parent is BattlementLayoutContainer layout)
            {
                layout.Adapter.Detach(child);
                return;
            }
            tabControls.Remove(child);
        }

        private void ApplyStickyAfterAttachment(UnityEngine.UIElements.VisualElement target)
        {
            if (!BattlementStickyItems.HasAuthored(target))
                return;
            stickyCoordinator.Apply(
                target,
                Prop<Sticky>.Set(BattlementStickyItems.Get(target)),
                hierarchy.SourceOrdinal(target)
            );
        }

        private void ApplyStickySubtree(UnityEngine.UIElements.VisualElement target)
        {
            ApplyStickyAfterAttachment(target);
            if (!hierarchy.TryGetId(target, out Guid id))
                return;
            foreach (Guid child in hierarchy.Children(id))
            {
                if (
                    hierarchy.TryGet(new ObjectId(child), out VisualElement? value)
                    && value is not null
                )
                    ApplyStickySubtree(value);
            }
        }

        private void RefreshStickyOrdinals() =>
            stickyCoordinator.RefreshOrdinals(hierarchy.SourceOrdinal);

        private void ApplyOverlayAfterAttachment(UnityEngine.UIElements.VisualElement target)
        {
            if (!BattlementOverlayItems.HasAuthored(target))
                return;
            if (!hierarchy.TryGetId(target, out Guid id))
                throw Failure(CoreErrorCode.InvalidHierarchy, "Overlay wrapper is not registered.");
            overlayCoordinator.Validate(
                new ObjectId(id),
                BattlementOverlayItems.Get(target),
                target.hierarchy.parent
                    ?? throw Failure(
                        CoreErrorCode.InvalidHierarchy,
                        "Overlay wrapper is not attached."
                    ),
                hierarchy.IsDescendant
            );
            overlayCoordinator.Apply(
                target,
                Prop<OverlayPlacement>.Set(BattlementOverlayItems.Get(target))
            );
        }

        private void ApplyOverlaySubtree(UnityEngine.UIElements.VisualElement target)
        {
            ApplyOverlayAfterAttachment(target);
            if (!hierarchy.TryGetId(target, out Guid id))
                return;
            foreach (Guid child in hierarchy.Children(id))
            {
                if (
                    hierarchy.TryGet(new ObjectId(child), out VisualElement? value)
                    && value is not null
                )
                    ApplyOverlaySubtree(value);
            }
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
            placementValidator.ValidateOverlayHost(parent);
            overlayCoordinator.Validate(
                objectId,
                element.OverlayPlacement.Value,
                parent,
                (candidate, ancestor) =>
                    pendingTree is not null && ContainsDetached(pendingTree, ancestor)
                        ? IsDetachedDescendant(pendingTree, candidate, ancestor)
                        : hierarchy.IsDescendant(candidate, ancestor),
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

        private bool IsOverlayScopeMember(
            UnityEngine.UIElements.VisualElement candidate,
            UnityEngine.UIElements.VisualElement scope
        ) =>
            candidate.panel == scope.panel
            && hierarchy.TryGetId(candidate, out Guid candidateId)
            && hierarchy.TryGetId(scope, out Guid scopeId)
            && hierarchy.IsDescendant(candidateId, scopeId);

        private IEnumerable<UnityEngine.UIElements.VisualElement> OverlayScopeTraversal(
            UnityEngine.UIElements.VisualElement scope
        )
        {
            if (!hierarchy.TryGetId(scope, out Guid scopeId))
                yield break;
            foreach (Guid id in hierarchy.LogicalPreorder(scopeId))
            {
                if (hierarchy.TryGet(new ObjectId(id), out VisualElement? value))
                    yield return value!;
            }
        }

        private IEnumerable<UnityEngine.UIElements.VisualElement> PhysicalOverlayScopeTraversal(
            UnityEngine.UIElements.VisualElement scope
        )
        {
            if (!hierarchy.TryGetId(scope, out Guid scopeId) || scope.panel is null)
                yield break;
            foreach (
                UnityEngine.UIElements.VisualElement candidate in PhysicalPreorder(
                    scope.panel.visualTree
                )
            )
            {
                if (
                    hierarchy.TryGetId(candidate, out Guid candidateId)
                    && hierarchy.IsDescendant(candidateId, scopeId)
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

        private static int SubtreeDepth(UiNode node)
        {
            int depth = 0;
            foreach (UiNode child in node.Children ?? Array.Empty<UiNode>())
                depth = Math.Max(depth, SubtreeDepth(child) + 1);
            return depth;
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
                BattlementUiPlacementValidator.ValidateNativeLayoutStyle(child, "Grid");
            bool parentIsStack =
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack };
            if (BattlementStackItems.HasAuthored(child) && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                BattlementUiPlacementValidator.ValidateNativeLayoutStyle(child, "Stack");
            if (
                BattlementStickyItems.HasAuthored(child)
                && !BattlementUiPlacementValidator.HasScrollAncestor(parent)
            )
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "Sticky requires a physical ScrollView ancestor."
                );
        }

        private static void ValidateGridPlacement(UiElement child, bool parentIsGrid)
        {
            if (child.GridItem.IsSet && !parentIsGrid)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "GridItem requires a direct Grid placement context."
                );
            if (parentIsGrid)
                BattlementUiPlacementValidator.ValidateLayoutStyle(child.Style, "Grid");
        }

        private static void ValidateStackPlacement(UiElement child, bool parentIsStack)
        {
            if (child.StackItem.IsSet && !parentIsStack)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "StackItem requires a direct Stack placement context."
                );
            if (parentIsStack)
                BattlementUiPlacementValidator.ValidateLayoutStyle(child.Style, "Stack");
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
            if (
                hierarchy.TryGetEntry(objectId, out BattlementUiHierarchy.Entry? entry)
                && entry is not null
            )
                RemoveIdentity(entry);
        }

        private void RemoveIdentity(BattlementUiHierarchy.Entry entry)
        {
            Guid objectId = entry.Id;
            UnityEngine.UIElements.VisualElement value = entry.Element;
            syntheticInput.RemoveIdentity(objectId);
            stickyCoordinator.Remove(value);
            overlayCoordinator.Remove(value);
            focusCoordinator.Remove(value);
            if (value is BattlementLayoutContainer layout)
                layout.Adapter.Clear();
            actions.Remove(new ObjectId(objectId), value);
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
            BattlementPaintProperties.Release(value);
            properties.Remove(objectId);
            scrollControls.Remove(objectId);
            repeatControls.Remove(objectId);
            hierarchy.Remove(objectId);
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
