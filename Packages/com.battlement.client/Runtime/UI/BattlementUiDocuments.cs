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
        private readonly BattlementUiControlledPointerInput controlledPointerInput;
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
            controlledPointerInput = new BattlementUiControlledPointerInput(
                () => hierarchy.InputDocuments,
                element => hierarchy.TryGetId(element, out Guid id) ? new ObjectId(id) : null
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
                uiProjectionSpace: ProjectionSpace,
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

        private BattlementUiProjectionSpace ProjectionSpace(VisualElement element)
        {
            if (
                !hierarchy.TryGetId(element, out Guid id)
                || !hierarchy.TryGetGeometryTarget(
                    new ObjectId(id),
                    out _,
                    out _,
                    out UIDocument document
                )
                || element.panel is null
            )
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Layout projection requires an attached Battlement UI element."
                );
            PanelSettings panel = document.panelSettings;
            if (
                panel.renderMode == UnityEngine.UIElements.PanelRenderMode.WorldSpace
                || panel.targetTexture != null
            )
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Shared UI layout requires a physical display panel mapping."
                );
            double scale = element.panel.scaledPixelsPerPoint;
            if (!double.IsFinite(scale) || scale <= 0)
                throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Shared UI layout requires a positive panel scale."
                );
            return new BattlementUiProjectionSpace(
                new DisplayId(checked((uint)panel.targetDisplay)),
                scale
            );
        }

        internal void BindMotionEffects(IBattlementMotionEffects value) =>
            motionWorld.BindEffects(value);

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
                RestoreNativeMotion?.Invoke();
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

        /// <summary>Replaces tracked hierarchies from flattened native document views.</summary>
        public void Replace(
            IBattlementUiDocumentCollectionView descriptions,
            Func<ObjectId, GameObject?> resolveGameObject,
            bool preserveMotion = false
        )
        {
            var resolved = new (
                IBattlementUiDocumentView View,
                UiDocument Root,
                UIDocument Document
            )[descriptions.DocumentCount];
            var roots = new HashSet<VisualElement>();
            for (int index = 0; index < resolved.Length; index++)
            {
                IBattlementUiDocumentView view = descriptions.ReadDocument(index);
                UiDocument root = view.ReadRoot();
                GameObject? gameObject = resolveGameObject(view.DocumentId);
                if (gameObject == null || !gameObject.TryGetComponent(out UIDocument document))
                    throw new InvalidOperationException(
                        $"UI document {view.DocumentId} has no owning UIDocument."
                    );
                if (!roots.Add(document.rootVisualElement))
                    throw new InvalidOperationException(
                        "An authoritative snapshot cannot assign one UIDocument to multiple roots."
                    );
                resolved[index] = (view, root, document);
            }

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
                foreach (
                    (
                        IBattlementUiDocumentView view,
                        UiDocument description,
                        UIDocument document
                    ) in resolved
                )
                {
                    VisualElement root = document.rootVisualElement;
                    root.Clear();
                    properties.CaptureDefaults(root, view.RootId);
                    properties.ApplyRoot(root, view.RootId, description);
                    hierarchy.AddRoot(view.RootId, root, document, RegisterHierarchyEntry);
                    focusCoordinator.ApplyRoot(root, description);
                    eventObserver.RegisterRoot(root);
                    PopulateFlatDocument(view, root);
                }
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
                accessibility.Refresh();
                lifecycleEvents.SetInputEnabled(true);
                RestoreNativeMotion?.Invoke();
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

        private void PopulateFlatDocument(IBattlementUiDocumentView description, VisualElement root)
        {
            var indices = new Dictionary<Guid, int>(description.NodeCount);
            var parents = new Dictionary<Guid, Guid>(description.NodeCount);
            for (int index = 0; index < description.NodeCount; index++)
                indices.Add(description.ReadNodeId(index).Value, index);
            for (int index = 0; index < description.NodeCount; index++)
            {
                Guid parent = description.ReadNodeId(index).Value;
                for (int child = 0; child < description.ReadChildCount(index); child++)
                    parents.Add(description.ReadChildId(index, child).Value, parent);
            }

            var order = new List<int>(description.NodeCount);
            var pending = new Stack<Guid>();
            for (int index = description.RootChildCount - 1; index >= 0; index--)
                pending.Push(description.ReadRootChildId(index).Value);
            while (pending.Count != 0)
            {
                int index = indices[pending.Pop()];
                order.Add(index);
                for (int child = description.ReadChildCount(index) - 1; child >= 0; child--)
                    pending.Push(description.ReadChildId(index, child).Value);
            }
            if (order.Count != description.NodeCount)
                throw new InvalidOperationException("A UI document forest is disconnected.");

            var created = new Dictionary<Guid, VisualElement>(order.Count);
            foreach (int index in order)
            {
                ObjectId id = description.ReadNodeId(index);
                UiElement element = description.ReadNodeElement(index);
                Guid parentId = parents.TryGetValue(id.Value, out Guid nestedParent)
                    ? nestedParent
                    : description.RootId.Value;
                VisualElement parent =
                    parentId == description.RootId.Value ? root : created[parentId];
                VisualElement value = CreateElement(
                    id,
                    element,
                    description.ReadChildCount(index),
                    description.RootId.Value,
                    parentId
                );
                created.Add(id.Value, value);
                InsertNativeChild(parent, value, null);
                hierarchy.AddChild(new ObjectId(parentId), id, hierarchy.Children(parentId).Count);
            }
            foreach (int index in order)
                InitializeCreatedContainer(
                    created[description.ReadNodeId(index).Value],
                    description.ReadNodeId(index),
                    description.ReadNodeElement(index)
                );
            for (int index = 0; index < description.RootChildCount; index++)
            {
                VisualElement child = created[description.ReadRootChildId(index).Value];
                ApplyStickySubtree(child);
                ApplyOverlaySubtree(child);
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
        internal BattlementUiNavigation Navigation =>
            new(() => InputDocuments, focusCoordinator.ShowSemanticFocus);

        internal void SetWorldCaptureResolver(Func<int, bool> captured) =>
            eventObserver.WorldCaptured = captured;

        internal bool HasPointerModal() =>
            hierarchy.InputDocuments.Any(document =>
                document != null
                && document.isActiveAndEnabled
                && overlayCoordinator.ActiveModal(document.rootVisualElement.panel) != null
            );

        internal bool BlocksWorldPointer(int pointerId, UnityEngine.Vector2 screen)
        {
            foreach (UIDocument document in hierarchy.InputDocuments)
            {
                if (document == null || !document.isActiveAndEnabled)
                    continue;
                VisualElement root = document.rootVisualElement;
                if (root.panel == null)
                    continue;
                if (overlayCoordinator.ActiveModal(root.panel) != null)
                    return true;
                if (root.panel.GetCapturingElement(pointerId) != null)
                    return true;
                if (
                    document.panelSettings.renderMode
                    != UnityEngine.UIElements.PanelRenderMode.ScreenSpaceOverlay
                )
                    continue;
                UnityEngine.Vector2 point = new(screen.x, Screen.height - screen.y);
                if (root.panel.Pick(point / root.panel.scaledPixelsPerPoint) != null)
                    return true;
            }
            return false;
        }

        internal BattlementUiControlledPointerResult ProcessControlledPointer(
            int pointerId,
            UnityEngine.Vector2 position,
            int buttons,
            bool isPresent,
            bool isCancelled
        ) => controlledPointerInput.Process(pointerId, position, buttons, isPresent, isCancelled);

        internal void ResetControlledPointer() => controlledPointerInput.Reset();

        internal BattlementMotionWorld MotionWorldForTests => motionWorld;

        public IReadOnlyList<MotionEffectOccurrence> MotionEffectOccurrences =>
            motionWorld.EffectOccurrences;
        internal BattlementMotionWorld MotionWorld => motionWorld;
        internal System.Action? RestoreNativeMotion { get; set; }

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

        internal IBattlementCommandOperation? ApplyValue(
            ObjectId valueId,
            MotionValueOperationKind kind,
            MotionValue? value,
            ObjectId playbackId,
            uint generation,
            TransitionDefinition? transition,
            bool blocking = false
        ) =>
            motionWorld.ApplyValue(
                valueId,
                kind,
                value,
                playbackId,
                generation,
                transition,
                blocking
            );

        internal void Apply(MotionValuePlaybackOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionPlaybackOperation operation) => motionWorld.Apply(operation);

        internal void Apply(MotionControlledClockOperation operation) =>
            motionWorld.Apply(operation);

        internal void Apply(MotionControlOperation operation) => motionWorld.Apply(operation);

        internal IBattlementCommandOperation? ApplyControl(
            ObjectId controlId,
            MotionControlOperationKind kind,
            ObjectId playbackId,
            uint generation,
            MotionControlTarget? target,
            bool blocking = false
        ) => motionWorld.ApplyControl(controlId, kind, playbackId, generation, target, blocking);

        internal void Apply(MotionScopeOperation operation) => motionWorld.Apply(operation);

        internal IBattlementCommandOperation? ApplyScope(
            IBattlementMotionScopeView operation,
            bool blocking = false
        ) => motionWorld.ApplyScope(operation, blocking);

        internal void Apply(MotionDragControlOperation operation) => motionWorld.Apply(operation);

        internal void ApplyValuePlayback(
            ObjectId playbackId,
            uint generation,
            MotionPlaybackOperationKind kind,
            ulong micros,
            double number,
            MotionPlaybackDirection direction
        ) =>
            motionWorld.ApplyValuePlayback(playbackId, generation, kind, micros, number, direction);

        internal void ApplyPlayback(
            ObjectId descriptorId,
            ulong slot,
            uint generation,
            MotionPlaybackOperationKind kind,
            ulong micros,
            double number,
            MotionPlaybackDirection direction
        ) =>
            motionWorld.ApplyPlayback(
                descriptorId,
                slot,
                generation,
                kind,
                micros,
                number,
                direction
            );

        internal void ApplyControlledClock(ObjectId clockId, ulong micros, bool advance)
        {
            if (advance)
                motionWorld.AdvanceControlledClock(clockId, micros);
            else
                motionWorld.SetControlledClock(clockId, micros);
        }

        internal void ApplyDragControl(
            ObjectId controlId,
            int pointerId,
            MotionPointerDevice device,
            float x,
            float y,
            bool snapToCursor
        ) => motionWorld.ApplyDragControl(controlId, pointerId, device, x, y, snapToCursor);

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
            foreach (UIDocument document in hierarchy.InputDocuments)
                if (document != null && document.rootVisualElement.panel is IPanel panel)
                    BattlementPointerCaptureTransfer.ReleaseIneligible(
                        panel,
                        focusCoordinator.IsEffectivelyInert
                    );
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
            controlledPointerInput.Reset();
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

        internal void Apply(IBattlementAccessibilityUpdateView update) =>
            accessibility.Apply(update);

        /// <summary>Releases every tracked root and element identity.</summary>
        public void Clear()
        {
            syntheticInput.Clear();
            controlledPointerInput.Reset();
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
        public void Create(CommandBody.VisualElement.Create command) =>
            Create(command.ParentId, command.Node, command.ChildIndex);

        internal void Create(ObjectId parentId, UiNode node, uint? childIndex)
        {
            UnityEngine.UIElements.VisualElement parent = Require(parentId);
            RequireContainer(parent, parentId);
            ValidatePlacement(node.Element, parent);
            ValidateOverlayContexts(
                node,
                parent is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
            );
            ValidateStickySubtree(node, BattlementUiPlacementValidator.HasScrollAncestor(parent));
            if (
                parent is UnityEngine.UIElements.ToggleButtonGroup
                && hierarchy.Children(parentId.Value).Count >= 64
            )
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
            int index = childIndex is uint requested
                ? checked((int)requested)
                : hierarchy.Children(parentId.Value).Count;
            if (index > hierarchy.Children(parentId.Value).Count)
            {
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");
            }

            var ids = new HashSet<Guid>();
            ValidateDetached(node, ids, 0);
            ValidateOverlayPlacement(node.ObjectId, node.Element, parent, node);
            int parentDepth = hierarchy.DepthOf(parentId.Value);
            if (parentDepth + SubtreeDepth(node) + 1 > BattlementUiHierarchy.MaximumDepth)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
            var reserved = new List<Guid>(ids);
            reserveIdentities?.Invoke(reserved);
            try
            {
                Guid rootId = hierarchy.DocumentRoot(parentId.Value);
                UnityEngine.UIElements.VisualElement created = CreateElement(
                    node,
                    rootId,
                    parentId.Value
                );
                choiceControls.BeginHierarchyMutation(parentId);
                InsertNativeChild(parent, created, childIndex is null ? null : index);
                hierarchy.AddChild(parentId, node.ObjectId, index);
                ApplyStickySubtree(created);
                ApplyOverlaySubtree(created);
                RefreshStickyOrdinals();
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
                choiceControls.Insert(parentId, index, hierarchy.Children(parentId.Value).Count);
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

        internal void Create(
            ObjectId parentId,
            ObjectId rootId,
            IBattlementUiForestView forest,
            uint? childIndex
        )
        {
            UnityEngine.UIElements.VisualElement parent = Require(parentId);
            RequireContainer(parent, parentId);
            int existingChildren = hierarchy.Children(parentId.Value).Count;
            if (parent is UnityEngine.UIElements.ToggleButtonGroup && existingChildren >= 64)
                throw Failure(CoreErrorCode.LimitExceeded, "ToggleButtonGroup accepts 64 buttons.");
            int insertionIndex = childIndex is uint requested
                ? checked((int)requested)
                : existingChildren;
            if (insertionIndex > existingChildren)
                throw Failure(CoreErrorCode.InvalidHierarchy, "UI child index is out of range.");

            var nodeIndices = new Dictionary<Guid, int>(forest.NodeCount);
            var parents = new Dictionary<Guid, Guid>(forest.NodeCount);
            var ids = new List<Guid>(forest.NodeCount);
            for (int index = 0; index < forest.NodeCount; index++)
            {
                Guid id = forest.ReadNodeId(index).Value;
                if (!nodeIndices.TryAdd(id, index) || hierarchy.Contains(id))
                    throw Failure(CoreErrorCode.DuplicateId, $"UI identity {id} is duplicated.");
                ids.Add(id);
            }
            if (!nodeIndices.ContainsKey(rootId.Value))
                throw Failure(CoreErrorCode.InvalidHierarchy, "The UI subtree root is absent.");
            if (hierarchy.Count + ids.Count > 100_000)
                throw Failure(CoreErrorCode.LimitExceeded, "The UI identity limit was exceeded.");
            for (int index = 0; index < forest.NodeCount; index++)
            {
                Guid id = forest.ReadNodeId(index).Value;
                for (int child = 0; child < forest.ReadChildCount(index); child++)
                {
                    Guid childId = forest.ReadChildId(index, child).Value;
                    if (!nodeIndices.ContainsKey(childId) || !parents.TryAdd(childId, id))
                        throw Failure(
                            CoreErrorCode.InvalidHierarchy,
                            "The UI subtree topology is invalid."
                        );
                }
            }

            var order = new List<int>(forest.NodeCount);
            var pending = new Stack<(Guid Id, int Depth, bool HasScroll, bool ParentIsStack)>();
            pending.Push(
                (
                    rootId.Value,
                    0,
                    BattlementUiPlacementValidator.HasScrollAncestor(parent),
                    parent
                        is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack }
                )
            );
            while (pending.Count != 0)
            {
                (Guid id, int depth, bool hasScroll, bool parentIsStack) = pending.Pop();
                if (depth > BattlementUiHierarchy.MaximumDepth)
                    throw Failure(CoreErrorCode.LimitExceeded, "The UI hierarchy is too deep.");
                int index = nodeIndices[id];
                UiElement element = forest.ReadNodeElement(index);
                int childCount = forest.ReadChildCount(index);
                ValidateDetachedElement(element, childCount);
                if (depth == 0)
                    ValidatePlacement(element, parent);
                else
                    ValidatePlacement(element, forest.ReadNodeElement(nodeIndices[parents[id]]));
                if (element.OverlayPlacement.IsSet && !parentIsStack)
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "Overlay placement requires a direct OverlayHost Stack target."
                    );
                if (element.Sticky.IsSet && !hasScroll)
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "Sticky requires a physical ScrollView ancestor."
                    );
                order.Add(index);
                bool descendantsHaveScroll = hasScroll || element is UiElement.ScrollView;
                bool nodeIsStack = element is UiElement.Stack;
                for (int child = childCount - 1; child >= 0; child--)
                    pending.Push(
                        (
                            forest.ReadChildId(index, child).Value,
                            depth + 1,
                            descendantsHaveScroll,
                            nodeIsStack
                        )
                    );
            }
            if (order.Count != forest.NodeCount)
                throw Failure(CoreErrorCode.InvalidHierarchy, "The UI subtree is disconnected.");

            UiElement rootElement = forest.ReadNodeElement(nodeIndices[rootId.Value]);
            ValidateFlatOverlayPlacement(rootId, rootElement, parent, nodeIndices, parents);
            reserveIdentities?.Invoke(ids);
            try
            {
                Guid documentRoot = hierarchy.DocumentRoot(parentId.Value);
                var created = new Dictionary<Guid, UnityEngine.UIElements.VisualElement>(
                    order.Count
                );
                foreach (int index in order)
                {
                    ObjectId id = forest.ReadNodeId(index);
                    UiElement element = forest.ReadNodeElement(index);
                    Guid logicalParent = parents.TryGetValue(id.Value, out Guid nestedParent)
                        ? nestedParent
                        : parentId.Value;
                    UnityEngine.UIElements.VisualElement value = CreateElement(
                        id,
                        element,
                        forest.ReadChildCount(index),
                        documentRoot,
                        logicalParent
                    );
                    created.Add(id.Value, value);
                    if (!parents.TryGetValue(id.Value, out nestedParent))
                        continue;
                    UnityEngine.UIElements.VisualElement createdParent = created[nestedParent];
                    InsertNativeChild(createdParent, value, null);
                    hierarchy.AddChild(
                        new ObjectId(nestedParent),
                        id,
                        hierarchy.Children(nestedParent).Count
                    );
                }
                foreach (int index in order)
                    InitializeCreatedContainer(
                        created[forest.ReadNodeId(index).Value],
                        forest.ReadNodeId(index),
                        forest.ReadNodeElement(index)
                    );
                UnityEngine.UIElements.VisualElement root = created[rootId.Value];
                choiceControls.BeginHierarchyMutation(parentId);
                InsertNativeChild(parent, root, childIndex is null ? null : insertionIndex);
                hierarchy.AddChild(parentId, rootId, insertionIndex);
                ApplyStickySubtree(root);
                ApplyOverlaySubtree(root);
                RefreshStickyOrdinals();
                RefreshOverlayOrdinals();
                focusCoordinator.Refresh();
                choiceControls.Insert(parentId, insertionIndex, existingChildren + 1);
            }
            catch
            {
                foreach (Guid id in ids)
                    RemoveIdentity(id);
                releaseIdentities?.Invoke(ids);
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

        internal void Update(BattlementDirectVisualElementPlacement command)
        {
            UnityEngine.UIElements.VisualElement target = Require(command.ObjectId);
            if (command.ChangesParent)
            {
                ApplyParent(
                    target,
                    command.ObjectId,
                    command.ParentId
                        ?? throw Failure(
                            CoreErrorCode.InvalidHierarchy,
                            "A UI parent update has no parent UUID."
                        ),
                    command.ChildIndex
                );
                return;
            }
            ApplyIndex(
                target,
                command.ObjectId,
                command.ChildIndex
                    ?? throw Failure(
                        CoreErrorCode.InvalidHierarchy,
                        "A UI index update has no child index."
                    )
            );
        }

        internal void UpdateLabelText(ObjectId objectId, string text)
        {
            UnityEngine.UIElements.VisualElement target = Require(objectId);
            if (target is not UnityEngine.UIElements.Label label)
            {
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    $"UI element {objectId} is not a label."
                );
            }
            label.text = text;
        }

        internal IBattlementCommandOperation? UpdateProperties(
            ObjectId objectId,
            UiElement element
        ) => propertyUpdates.Apply(objectId, element);

        internal void UpdateScalar(IBattlementUiScalarUpdateView update)
        {
            UnityEngine.UIElements.VisualElement target = Require(update.ObjectId);
            switch (update.Kind)
            {
                case BattlementUiScalarUpdateKind.TextFieldValue:
                    string textValue = update.ReadText() ?? string.Empty;
                    textFieldControls.ApplyDirectValue(update.ObjectId, textValue);
                    break;
                case BattlementUiScalarUpdateKind.BooleanValue:
                    booleanControls.ApplyDirectValue(update.ObjectId, update.Boolean);
                    break;
                case BattlementUiScalarUpdateKind.RadioSelection:
                    choiceControls.ApplyDirectRadio(update.ObjectId, update.Unsigned);
                    break;
                case BattlementUiScalarUpdateKind.ToggleSelection:
                    var selected = new uint[update.IndexCount];
                    for (int index = 0; index < selected.Length; index++)
                        selected[index] = update.ReadIndex(index);
                    choiceControls.ApplyDirectToggle(update.ObjectId, selected);
                    break;
                case BattlementUiScalarUpdateKind.DropdownSelection:
                    string? choiceValue = update.ReadText();
                    dropdownControls.ApplyDirectSelection(
                        update.ObjectId,
                        update.Boolean ? update.Unsigned : null,
                        choiceValue
                    );
                    break;
                case BattlementUiScalarUpdateKind.ScrollerValue:
                    scrollControls.ApplyDirectValue(update.ObjectId, update.First);
                    break;
                case BattlementUiScalarUpdateKind.SliderValue:
                    sliderControls.ApplyDirectFloat(update.ObjectId, update.First);
                    break;
                case BattlementUiScalarUpdateKind.SliderIntValue:
                    sliderControls.ApplyDirectInt(update.ObjectId, update.Integer);
                    break;
                case BattlementUiScalarUpdateKind.RangeValue:
                    rangeControls.ApplyDirectRange(update.ObjectId, update.First, update.Second);
                    break;
                case BattlementUiScalarUpdateKind.TabSelection:
                    tabControls.ApplyDirectSelection(update.ObjectId, update.Unsigned);
                    break;
                case BattlementUiScalarUpdateKind.ButtonText:
                    string buttonText = update.ReadText() ?? string.Empty;
                    RequireButton(target, update.ObjectId).text = buttonText;
                    break;
                case BattlementUiScalarUpdateKind.ButtonEnabled:
                    RequireButton(target, update.ObjectId).SetEnabled(update.Boolean);
                    break;
                case BattlementUiScalarUpdateKind.ButtonTextAndEnabled:
                    UnityEngine.UIElements.Button button = RequireButton(target, update.ObjectId);
                    string enabledButtonText = update.ReadText() ?? string.Empty;
                    button.text = enabledButtonText;
                    button.SetEnabled(update.Boolean);
                    break;
                case BattlementUiScalarUpdateKind.RepeatTiming:
                    repeatControls.ApplyDirectTiming(
                        RequireRepeatButton(target, update.ObjectId),
                        update.ObjectId,
                        update.Unsigned,
                        update.UnsignedSecond
                    );
                    break;
                default:
                    throw new InvalidOperationException("Unknown direct UI scalar update.");
            }
        }

        private static UnityEngine.UIElements.Button RequireButton(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId
        ) =>
            target as UnityEngine.UIElements.Button
            ?? throw Failure(
                CoreErrorCode.InvalidProperty,
                $"UI element {objectId} is not a button."
            );

        private static UnityEngine.UIElements.RepeatButton RequireRepeatButton(
            UnityEngine.UIElements.VisualElement target,
            ObjectId objectId
        ) =>
            target as UnityEngine.UIElements.RepeatButton
            ?? throw Failure(
                CoreErrorCode.InvalidProperty,
                $"UI element {objectId} is not a repeat button."
            );

        /// <summary>Destroys one non-root element and its logical descendants.</summary>
        public void Destroy(CommandBody.VisualElement.Destroy command) => Destroy(command.ObjectId);

        internal void Destroy(ObjectId objectId)
        {
            UnityEngine.UIElements.VisualElement target = Require(objectId);
            if (hierarchy.IsRoot(objectId.Value))
            {
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "A document root cannot be destroyed by a UI command."
                );
            }
            Guid parentId =
                hierarchy.ParentId(objectId.Value)
                ?? throw new InvalidOperationException("A non-root UI element lost its parent.");
            int removedIndex = hierarchy.IndexOfChild(parentId, objectId.Value);
            choiceControls.BeginHierarchyMutation(new ObjectId(parentId));
            stickyCoordinator.PrepareHierarchyChange(target);
            RemoveNativeChild(Require(new ObjectId(parentId)), target);
            IReadOnlyList<BattlementUiHierarchy.Entry> removed = hierarchy.RemoveSubtree(objectId);
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

        internal void PerformAction(BattlementDirectVisualElementAction command) =>
            actions.Perform(command);

        private UnityEngine.UIElements.VisualElement CreateElement(
            UiNode node,
            Guid documentRoot,
            Guid parentId
        )
        {
            IReadOnlyList<UiNode> children = node.Children ?? Array.Empty<UiNode>();
            UnityEngine.UIElements.VisualElement value = CreateElement(
                node.ObjectId,
                node.Element,
                children.Count,
                documentRoot,
                parentId
            );
            PopulateChildren(value, node, children, documentRoot);
            return value;
        }

        private UnityEngine.UIElements.VisualElement CreateElement(
            ObjectId objectId,
            UiElement description,
            int childCount,
            Guid documentRoot,
            Guid parentId
        )
        {
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
                UiElement.ToggleButtonGroup toggle => CreateToggleButtonGroup(childCount, toggle),
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
                UiElement.RepeatButton repeat => repeatControls.Create(objectId, repeat),
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

            properties.CaptureDefaults(value, objectId);
            PopulateElement(value, objectId, description, documentRoot, parentId);
            value.RegisterCallback<UnityTransitionStartEvent>(eventValue =>
                events.ForwardTransition(
                    objectId,
                    UiEventKind.TransitionStart,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            value.RegisterCallback<UnityTransitionEndEvent>(eventValue =>
                events.ForwardTransition(
                    objectId,
                    UiEventKind.TransitionEnd,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            value.RegisterCallback<UnityTransitionCancelEvent>(eventValue =>
                events.ForwardTransition(
                    objectId,
                    UiEventKind.TransitionCancel,
                    eventValue.stylePropertyNames,
                    eventValue.elapsedTime
                )
            );
            return value;
        }

        private static UnityEngine.UIElements.ToggleButtonGroup CreateToggleButtonGroup(
            int childCount,
            UiElement.ToggleButtonGroup value
        )
        {
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

        private void PopulateElement(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId,
            UiElement element,
            Guid documentRoot,
            Guid parentId
        )
        {
            using BattlementPreparedMotionAdmission? preparedMotion = motionWorld.Prepare(
                value,
                objectId,
                element.Motion,
                element.Paint
            );
            properties.ApplyElement(value, objectId, element);
            BattlementPaintProperties.Apply(value, element.Paint);
            focusCoordinator.ApplyCreate(value, element);
            BattlementGridItems.Apply(value, element.GridItem);
            BattlementStackItems.Apply(value, element.StackItem);
            BattlementStickyItems.Apply(value, element.Sticky);
            BattlementOverlayItems.Apply(value, element.OverlayPlacement);
            if (value is BattlementLayoutContainer layout && element is UiElement.Flex flex)
                layout.ApplyFlex(flex);
            if (value is BattlementLayoutContainer gridLayout && element is UiElement.Grid grid)
                gridLayout.ApplyGrid(grid);
            if (value is BattlementLayoutContainer stackLayout && element is UiElement.Stack stack)
                stackLayout.ApplyStack(stack);
            Reserve(objectId, value, documentRoot, parentId);
            scrollControls.ApplyCreate(value, objectId, element);
            tabControls.ApplyCreate(value, objectId, element);
            textFieldControls.ApplyCreate(value, objectId, element);
            booleanControls.ApplyCreate(value, objectId, element);
            choiceControls.ApplyCreate(value, objectId, element);
            dropdownControls.ApplyCreate(value, objectId, element);
            sliderControls.ApplyCreate(value, objectId, element);
            rangeControls.ApplyCreate(value, objectId, element);
            partProperties.Apply(value, objectId, element);
            preparedMotion?.Commit();
        }

        private void PopulateChildren(
            UnityEngine.UIElements.VisualElement value,
            UiNode node,
            IReadOnlyList<UiNode> children,
            Guid documentRoot
        )
        {
            BattlementLayoutContainer? updatingLayout = value as BattlementLayoutContainer;
            updatingLayout?.BeginUpdate();
            try
            {
                foreach (UiNode child in children)
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
            InitializeCreatedContainer(value, node.ObjectId, node.Element);
        }

        private void InitializeCreatedContainer(
            UnityEngine.UIElements.VisualElement value,
            ObjectId objectId,
            UiElement element
        )
        {
            if (element is UiElement.TabView tabView)
                tabControls.Initialize(
                    (UnityEngine.UIElements.TabView)value,
                    objectId,
                    tabView.SelectedTabIndex
                );
            if (element is UiElement.ToggleButtonGroup)
                choiceControls.InitializeToggle(objectId, hierarchy.Children(objectId.Value).Count);
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
            IReadOnlyList<UiNode> children = node.Children ?? Array.Empty<UiNode>();
            ValidateDetachedElement(node.Element, children.Count);
            foreach (UiNode child in children)
            {
                ValidatePlacement(child.Element, node.Element);
                ValidateDetached(child, ids, depth + 1);
            }
        }

        private static void ValidateDetachedElement(UiElement element, int childCount)
        {
            BattlementUiElementProperties.Validate(element, allowUsageHints: true);
            if (
                element
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
                && childCount != 0
            )
                throw Failure(
                    CoreErrorCode.InvalidHierarchy,
                    "Leaf UI controls cannot contain logical children."
                );
            if (
                element is UiElement.TabView tabView
                && tabView.SelectedTabIndex.IsSet
                && tabView.SelectedTabIndex.Value >= childCount
            )
                throw Failure(CoreErrorCode.InvalidProperty, "Selected tab index is out of range.");
            BattlementUiChoiceControls.ValidateNode(element, childCount);
            BattlementUiDropdownControls.ValidateNode(element);
            BattlementUiSliderControls.ValidateNode(element);
            BattlementUiRangeControls.ValidateNode(element);
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
            using (new BattlementPointerCaptureTransfer(target))
            {
                RemoveNativeChild(Require(new ObjectId(plan.OldParentId)), target);
                InsertNativeChild(parent, target, plan.NewIndex);
            }
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
            using var captureTransfer = new BattlementPointerCaptureTransfer(target);
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

        private void ValidateFlatOverlayPlacement(
            ObjectId objectId,
            UiElement element,
            UnityEngine.UIElements.VisualElement parent,
            IReadOnlyDictionary<Guid, int> nodes,
            IReadOnlyDictionary<Guid, Guid> parents
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
                    nodes.ContainsKey(ancestor)
                        ? FlatIsDescendant(candidate, ancestor, parents)
                        : hierarchy.IsDescendant(candidate, ancestor),
                id => nodes.ContainsKey(id.Value)
            );
        }

        private static bool FlatIsDescendant(
            Guid candidate,
            Guid ancestor,
            IReadOnlyDictionary<Guid, Guid> parents
        )
        {
            Guid current = candidate;
            while (parents.TryGetValue(current, out Guid parent))
            {
                if (parent == ancestor)
                    return true;
                current = parent;
            }
            return false;
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
