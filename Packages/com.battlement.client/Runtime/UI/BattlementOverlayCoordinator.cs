#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using SystemAction = System.Action;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal readonly struct BattlementPopoverResult
    {
        public BattlementPopoverResult(UnityRect rect, PlacementSide side)
        {
            Rect = rect;
            Side = side;
        }

        public UnityRect Rect { get; }
        public PlacementSide Side { get; }
    }

    internal sealed class BattlementOverlayCoordinator
    {
        private readonly Dictionary<VisualElement, Entry> entries = new();
        private readonly Dictionary<VisualElement, Surface> surfaces = new();
        private readonly Func<ObjectId, VisualElement?> resolve;
        private readonly Func<VisualElement, int> sourceOrdinal;
        private readonly Func<VisualElement, VisualElement, bool> isScopeMember;
        private readonly Func<VisualElement, IEnumerable<VisualElement>> scopeTraversal;
        private readonly SystemAction focusBoundaryChanged;

        public BattlementOverlayCoordinator(
            Func<ObjectId, VisualElement?> resolve,
            Func<VisualElement, int> sourceOrdinal,
            Func<VisualElement, VisualElement, bool> isScopeMember,
            Func<VisualElement, IEnumerable<VisualElement>> scopeTraversal,
            SystemAction? focusBoundaryChanged = null
        )
        {
            this.resolve = resolve;
            this.sourceOrdinal = sourceOrdinal;
            this.isScopeMember = isScopeMember;
            this.scopeTraversal = scopeTraversal;
            this.focusBoundaryChanged = focusBoundaryChanged ?? (() => { });
        }

        public int UpdateCount { get; private set; }

        public int EntryCount => entries.Count;

        public VisualElement? ActiveModal(IPanel? panel) =>
            surfaces
                .Values.FirstOrDefault(value => value.Host.panel == panel)
                ?.ActiveModal?.Wrapper;

        public void Validate(
            ObjectId targetId,
            OverlayPlacement placement,
            VisualElement physicalParent,
            Func<Guid, Guid, bool> isDescendant
        )
        {
            VisualElement host = RequireOverlayHost(physicalParent);
            foreach (VisualElement other in surfaces.Keys)
            {
                if (other != host && other.panel != null && other.panel == host.panel)
                    throw Failure(
                        $"Overlay host {targetId} conflicts with another host in this panel."
                    );
            }
            switch (placement)
            {
                case OverlayPlacement.Layer:
                    break;
                case OverlayPlacement.Popover popover:
                    VisualElement? anchor = resolve(popover.Anchor);
                    if (anchor is null)
                        return;
                    if (anchor.panel != null && host.panel != null && anchor.panel != host.panel)
                        throw Failure(
                            $"Popover {targetId} and anchor {popover.Anchor} "
                                + "belong to different panels."
                        );
                    if (
                        popover.Anchor == targetId
                        || isDescendant(popover.Anchor.Value, targetId.Value)
                    )
                        throw Failure(
                            $"Popover {targetId} cannot anchor to itself or a logical "
                                + $"descendant {popover.Anchor}."
                        );
                    break;
                case OverlayPlacement.Modal modal:
                    ValidateFocusRef(
                        targetId,
                        modal.InitialFocus,
                        requireDescendant: true,
                        isDescendant
                    );
                    ValidateFocusRef(
                        targetId,
                        modal.RestoreFocus,
                        requireDescendant: false,
                        isDescendant
                    );
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(placement));
            }
        }

        public void Apply(VisualElement target, Prop<OverlayPlacement> value)
        {
            BattlementOverlayItems.Apply(target, value);
            if (value.IsReset)
            {
                Detach(target, restoreFocus: true);
                return;
            }
            if (!BattlementOverlayItems.HasAuthored(target))
                return;
            if (entries.TryGetValue(target, out Entry existing))
            {
                existing.Placement = BattlementOverlayItems.Get(target);
                existing.SourceOrdinal = sourceOrdinal(target);
                existing.RebindAnchor(resolve);
                RefreshAll();
                return;
            }
            Attach(target);
        }

        public void PrepareHierarchyChange(VisualElement target) =>
            Detach(target, restoreFocus: false);

        public void RefreshOrdinals()
        {
            foreach (Entry entry in entries.Values)
                entry.SourceOrdinal = sourceOrdinal(entry.Wrapper);
            RefreshAll();
        }

        public void Remove(VisualElement target) => Detach(target, restoreFocus: true);

        public void Clear()
        {
            foreach (VisualElement target in entries.Keys.ToArray())
                Detach(target, restoreFocus: false);
        }

        public void RefreshAll()
        {
            foreach (Surface surface in surfaces.Values)
                Refresh(surface);
            foreach (Surface surface in surfaces.Values)
                RefreshFocus(surface);
        }

        internal static BattlementPopoverResult ResolvePopover(
            UnityRect host,
            UnityRect anchor,
            Vector2 size,
            PopoverPlacement placement
        )
        {
            float paddingX = Math.Min(placement.CollisionPadding, host.width / 2);
            float paddingY = Math.Min(placement.CollisionPadding, host.height / 2);
            UnityRect padded = UnityRect.MinMaxRect(
                host.xMin + paddingX,
                host.yMin + paddingY,
                host.xMax - paddingX,
                host.yMax - paddingY
            );
            UnityRect requested = Place(anchor, size, placement.Side, placement.Align, placement);
            PlacementSide opposite = Opposite(placement.Side);
            UnityRect selected = requested;
            PlacementSide side = placement.Side;
            if (placement.Flip)
            {
                UnityRect candidate = Place(anchor, size, opposite, placement.Align, placement);
                if (
                    MainOverflow(candidate, padded, opposite)
                    < MainOverflow(requested, padded, side)
                )
                {
                    selected = candidate;
                    side = opposite;
                }
            }
            if (placement.Shift)
                selected = ShiftCross(selected, padded, side);
            return new BattlementPopoverResult(selected, side);
        }

        private void Attach(VisualElement target)
        {
            VisualElement physicalParent =
                target.hierarchy.parent
                ?? throw Failure("Overlay wrapper is not physically attached.");
            VisualElement host = RequireOverlayHost(physicalParent);
            Surface surface = GetSurface(host);
            var entry = new Entry(
                target,
                surface,
                BattlementOverlayItems.Get(target),
                sourceOrdinal(target),
                RefreshAll
            );
            entries.Add(target, entry);
            surface.Entries.Add(entry);
            entry.RebindAnchor(resolve);
            RefreshAll();
        }

        private void Detach(VisualElement target, bool restoreFocus)
        {
            if (!entries.Remove(target, out Entry entry))
                return;
            bool containedFocus = ContainsFocus(entry);
            entry.Dispose();
            entry.Surface.Entries.Remove(entry);
            if (
                restoreFocus
                && containedFocus
                && entry.Placement is OverlayPlacement.Popover popover
            )
                FocusIfEligible(resolve(popover.Anchor));
            if (
                restoreFocus
                && entry == entry.Surface.ActiveModal
                && entry.Placement is OverlayPlacement.Modal modal
            )
                entry.Surface.PendingRestore = modal.RestoreFocus is ObjectId id
                    ? resolve(id)
                    : null;
            if (entry.Surface.Entries.Count == 0)
            {
                if (restoreFocus)
                    RefreshFocus(entry.Surface);
                entry.Surface.Dispose();
                surfaces.Remove(entry.Surface.Host);
                return;
            }
            RefreshAll();
        }

        private Surface GetSurface(VisualElement host)
        {
            if (surfaces.TryGetValue(host, out Surface surface))
                return surface;
            surface = new Surface(host, RefreshAll, OnFocusIn, OnKeyDown);
            surfaces.Add(host, surface);
            return surface;
        }

        private void Refresh(Surface surface)
        {
            surface.BindPanel();
            surface.Entries.Sort(EntryComparer.Instance);
            foreach (Entry entry in surface.Entries)
                Present(entry);
            if (surface.Host is BattlementLayoutContainer stack)
                stack.StackLayout?.Invalidate();
        }

        private void Present(Entry entry)
        {
            SetHostFill(entry, entry.Placement is not OverlayPlacement.Popover);
            if (entry.Placement is OverlayPlacement.Popover popover)
            {
                VisualElement? anchor = resolve(popover.Anchor);
                entry.RebindAnchor(resolve);
                if (
                    !GeometryReady(entry.Surface.Host)
                    || !entry.WrapperCurrent
                    || !GeometryReady(entry.Wrapper)
                    || !EligibleAnchor(anchor)
                )
                {
                    SetWaiting(entry, true, anchor);
                    return;
                }
                BattlementPopoverResult result = ResolvePopover(
                    entry.Surface.Host.worldBound,
                    anchor!.worldBound,
                    new Vector2(entry.Wrapper.layout.width, entry.Wrapper.layout.height),
                    popover.Placement
                );
                BattlementLayoutSlot slot = RequireSlot(entry.Wrapper);
                slot.style.left = result.Rect.x - entry.Surface.Host.worldBound.x;
                slot.style.top = result.Rect.y - entry.Surface.Host.worldBound.y;
                slot.style.width = result.Rect.width;
                slot.style.height = result.Rect.height;
                SetWaiting(entry, false, anchor);
            }
            else
            {
                BattlementLayoutSlot slot = RequireSlot(entry.Wrapper);
                slot.style.left = 0;
                slot.style.top = 0;
                slot.style.width = Length.Percent(100);
                slot.style.height = Length.Percent(100);
                SetWaiting(entry, false, null);
            }
            UpdateCount++;
        }

        private static void SetHostFill(Entry entry, bool fill)
        {
            if (entry.HostFill == fill)
                return;
            entry.HostFill = fill;
            if (fill)
            {
                entry.Wrapper.style.position = Position.Absolute;
                entry.Wrapper.style.top = 0;
                entry.Wrapper.style.right = 0;
                entry.Wrapper.style.bottom = 0;
                entry.Wrapper.style.left = 0;
                return;
            }
            entry.Wrapper.style.position = StyleKeyword.Null;
            entry.Wrapper.style.top = StyleKeyword.Null;
            entry.Wrapper.style.right = StyleKeyword.Null;
            entry.Wrapper.style.bottom = StyleKeyword.Null;
            entry.Wrapper.style.left = StyleKeyword.Null;
        }

        private void RefreshFocus(Surface surface)
        {
            Entry? next = null;
            foreach (Entry candidate in surface.Entries)
            {
                if (candidate.Placement is not OverlayPlacement.Modal)
                    continue;
                if (next is null || EntryComparer.Instance.Compare(next, candidate) < 0)
                    next = candidate;
            }
            if (next == surface.ActiveModal)
            {
                if (next is not null && !next.FocusActivated && PresentationReady(next))
                    FocusModal(next);
                return;
            }
            if (surface.ActiveModal is not null && ContainsFocus(surface.ActiveModal))
                surface.ActiveModal.LastFocused =
                    surface.ActiveModal.Wrapper.panel?.focusController.focusedElement
                    as VisualElement;
            if (surface.ActiveModal is null && next is not null)
                surface.ApplicationReturn =
                    next.Wrapper.panel?.focusController.focusedElement as VisualElement;
            surface.ActiveModal = next;
            focusBoundaryChanged();
            if (next is null)
            {
                if (IsFocusEligible(surface.PendingRestore))
                    surface.PendingRestore!.Focus();
                else
                    FocusIfEligible(surface.ApplicationReturn);
                surface.PendingRestore = null;
                surface.ApplicationReturn = null;
                return;
            }
            FocusModal(next);
        }

        private void FocusModal(Entry entry)
        {
            if (!PresentationReady(entry))
                return;
            if (
                IsFocusEligible(entry.LastFocused)
                && isScopeMember(entry.LastFocused!, entry.Wrapper)
            )
            {
                FocusRequired(entry, entry.LastFocused!);
                return;
            }
            var modal = (OverlayPlacement.Modal)entry.Placement;
            VisualElement? requested = modal.InitialFocus is ObjectId id ? resolve(id) : null;
            if (
                requested is not null
                && IsFocusEligible(requested)
                && isScopeMember(requested, entry.Wrapper)
            )
            {
                FocusRequired(entry, requested);
                entry.LastFocused = requested;
                entry.FocusActivated = true;
                return;
            }
            VisualElement? fallback = SequentialTraversal(entry).FirstOrDefault();
            FocusRequired(entry, fallback ?? entry.Wrapper);
            entry.LastFocused = fallback ?? entry.Wrapper;
            entry.FocusActivated = true;
        }

        private void OnFocusIn(FocusInEvent eventValue)
        {
            if (eventValue.target is not VisualElement target)
                return;
            Surface? surface = SurfaceFor(target);
            if (surface?.ActiveModal is not Entry activeModal)
                return;
            if (ScopeHost(target, activeModal) is VisualElement focused)
            {
                activeModal.LastFocused = focused;
                return;
            }
            FocusModal(activeModal);
        }

        private void OnKeyDown(KeyDownEvent eventValue)
        {
            if (eventValue.target is not VisualElement target || eventValue.keyCode != KeyCode.Tab)
                return;
            Surface? surface = SurfaceFor(target);
            if (surface?.ActiveModal is not Entry activeModal)
                return;
            VisualElement[] traversal = SequentialTraversal(activeModal).ToArray();
            if (traversal.Length == 0)
            {
                activeModal.Wrapper.Focus();
                ConsumeTab(activeModal, eventValue);
                return;
            }
            VisualElement? focused =
                activeModal.Wrapper.panel?.focusController.focusedElement as VisualElement;
            int index = Array.IndexOf(traversal, focused);
            bool boundary = eventValue.shiftKey
                ? index <= 0
                : index < 0 || index == traversal.Length - 1;
            if (!boundary)
                return;
            int next = eventValue.shiftKey ? traversal.Length - 1 : 0;
            traversal[next].Focus();
            activeModal.LastFocused = traversal[next];
            ConsumeTab(activeModal, eventValue);
        }

        private static void ConsumeTab(Entry activeModal, KeyDownEvent eventValue)
        {
            activeModal.Wrapper.panel?.focusController.IgnoreEvent(eventValue);
            eventValue.StopPropagation();
        }

        private Surface? SurfaceFor(VisualElement target) =>
            surfaces.Values.FirstOrDefault(value => value.Host.panel == target.panel);

        private sealed class EntryComparer : IComparer<Entry>
        {
            public static readonly EntryComparer Instance = new();

            public int Compare(Entry? left, Entry? right)
            {
                if (ReferenceEquals(left, right))
                    return 0;
                if (left is null)
                    return -1;
                if (right is null)
                    return 1;
                int order = BattlementStackItems
                    .Get(left.Wrapper)
                    .Order.CompareTo(BattlementStackItems.Get(right.Wrapper).Order);
                return order != 0 ? order : left.SourceOrdinal.CompareTo(right.SourceOrdinal);
            }
        }

        private void ValidateFocusRef(
            ObjectId wrapper,
            ObjectId? reference,
            bool requireDescendant,
            Func<Guid, Guid, bool> isDescendant
        )
        {
            if (reference is not ObjectId id)
                return;
            if (resolve(id) is not VisualElement target)
                throw Failure($"Modal {wrapper} focus ref {id} does not exist.");
            if (requireDescendant && !isDescendant(id.Value, wrapper.Value))
                throw Failure($"Modal {wrapper} initial focus {id} is outside its scope.");
            VisualElement? wrapperElement = resolve(wrapper);
            if (
                wrapperElement?.panel != null
                && target.panel != null
                && wrapperElement.panel != target.panel
            )
                throw Failure($"Modal {wrapper} focus ref {id} belongs to another panel.");
        }

        private static UnityRect Place(
            UnityRect anchor,
            Vector2 size,
            PlacementSide side,
            PlacementAlign align,
            PopoverPlacement placement
        )
        {
            float cross = side is PlacementSide.Top or PlacementSide.Bottom
                ? Align(anchor.xMin, anchor.width, size.x, align) + placement.CrossOffset
                : Align(anchor.yMin, anchor.height, size.y, align) + placement.CrossOffset;
            return side switch
            {
                PlacementSide.Top => new UnityRect(
                    cross,
                    anchor.yMin - size.y - placement.MainOffset,
                    size.x,
                    size.y
                ),
                PlacementSide.Right => new UnityRect(
                    anchor.xMax + placement.MainOffset,
                    cross,
                    size.x,
                    size.y
                ),
                PlacementSide.Bottom => new UnityRect(
                    cross,
                    anchor.yMax + placement.MainOffset,
                    size.x,
                    size.y
                ),
                PlacementSide.Left => new UnityRect(
                    anchor.xMin - size.x - placement.MainOffset,
                    cross,
                    size.x,
                    size.y
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(side)),
            };
        }

        private static float Align(
            float start,
            float anchorSize,
            float size,
            PlacementAlign align
        ) =>
            align switch
            {
                PlacementAlign.Center => start + (anchorSize - size) / 2,
                PlacementAlign.End => start + anchorSize - size,
                _ => start,
            };

        private static float MainOverflow(UnityRect value, UnityRect bounds, PlacementSide side) =>
            side is PlacementSide.Top or PlacementSide.Bottom
                ? Math.Max(0, bounds.yMin - value.yMin) + Math.Max(0, value.yMax - bounds.yMax)
                : Math.Max(0, bounds.xMin - value.xMin) + Math.Max(0, value.xMax - bounds.xMax);

        private static UnityRect ShiftCross(UnityRect value, UnityRect bounds, PlacementSide side)
        {
            if (side is PlacementSide.Top or PlacementSide.Bottom)
                value.x =
                    value.width > bounds.width
                        ? bounds.xMin
                        : Math.Min(Math.Max(value.x, bounds.xMin), bounds.xMax - value.width);
            else
                value.y =
                    value.height > bounds.height
                        ? bounds.yMin
                        : Math.Min(Math.Max(value.y, bounds.yMin), bounds.yMax - value.height);
            return value;
        }

        private static PlacementSide Opposite(PlacementSide side) =>
            side switch
            {
                PlacementSide.Top => PlacementSide.Bottom,
                PlacementSide.Right => PlacementSide.Left,
                PlacementSide.Bottom => PlacementSide.Top,
                PlacementSide.Left => PlacementSide.Right,
                _ => throw new ArgumentOutOfRangeException(nameof(side)),
            };

        private static VisualElement RequireOverlayHost(VisualElement parent) =>
            parent switch
            {
                BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Stack } stack =>
                    stack,
                BattlementLayoutSlot
                {
                    ContainingBlock: BattlementLayoutContainer
                    {
                        Kind: BattlementLayoutContainerKind.Stack
                    } stack
                } => stack,
                _ => throw Failure("Overlay placement requires a direct OverlayHost Stack target."),
            };

        private static BattlementLayoutSlot RequireSlot(VisualElement wrapper) =>
            wrapper.hierarchy.parent as BattlementLayoutSlot
            ?? throw Failure("Overlay wrapper lost its private Stack slot.");

        private static bool GeometryReady(VisualElement value) =>
            value.panel != null
            && float.IsFinite(value.layout.width)
            && float.IsFinite(value.layout.height);

        private static bool PresentationReady(Entry entry) =>
            entry.WrapperCurrent && GeometryReady(entry.Wrapper);

        private static bool EligibleAnchor(VisualElement? value) =>
            value is not null
            && GeometryReady(value)
            && value.resolvedStyle.display != DisplayStyle.None
            && value.resolvedStyle.visibility == Visibility.Visible;

        private static bool IsFocusEligible(VisualElement? value) =>
            value is not null
            && value.panel != null
            && value.focusable
            && value.enabledInHierarchy
            && value.resolvedStyle.display != DisplayStyle.None
            && value.resolvedStyle.visibility == Visibility.Visible;

        private IEnumerable<VisualElement> Traversal(Entry entry) =>
            scopeTraversal(entry.Wrapper)
                .Where(value =>
                    value != entry.Wrapper && value.tabIndex >= 0 && IsFocusEligible(value)
                );

        private IEnumerable<VisualElement> SequentialTraversal(Entry entry) =>
            Traversal(entry).OrderBy(value => value.tabIndex > 0 ? value.tabIndex : int.MaxValue);

        private bool ContainsFocus(Entry entry) =>
            entry.Wrapper.panel?.focusController.focusedElement is VisualElement focused
            && ScopeHost(focused, entry) is not null;

        private VisualElement? ScopeHost(VisualElement focused, Entry entry)
        {
            for (VisualElement? current = focused; current is not null; current = current.parent)
            {
                if (current == entry.Wrapper || isScopeMember(current, entry.Wrapper))
                    return current;
            }
            return null;
        }

        private void FocusRequired(Entry entry, VisualElement target)
        {
            target.Focus();
            if (!ContainsFocus(entry))
                throw Failure("An active modal must retain focus inside its scope.");
        }

        private static void FocusIfEligible(VisualElement? value)
        {
            if (IsFocusEligible(value))
                value!.Focus();
        }

        private void SetWaiting(Entry entry, bool waiting, VisualElement? anchor)
        {
            if (entry.Waiting == waiting)
                return;
            bool restore = waiting && ContainsFocus(entry);
            entry.Waiting = waiting;
            entry.Wrapper.style.visibility = waiting ? Visibility.Hidden : Visibility.Visible;
            entry.Wrapper.pickingMode = PickingMode.Ignore;
            if (restore)
                FocusIfEligible(anchor);
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class Surface : IDisposable
        {
            private readonly EventCallback<FocusInEvent> focusIn;
            private readonly EventCallback<KeyDownEvent> keyDown;
            private readonly EventCallback<GeometryChangedEvent> geometry;
            private VisualElement? panelRoot;

            public Surface(
                VisualElement host,
                SystemAction refresh,
                EventCallback<FocusInEvent> focusIn,
                EventCallback<KeyDownEvent> keyDown
            )
            {
                Host = host;
                Entries = new List<Entry>();
                this.focusIn = focusIn;
                this.keyDown = keyDown;
                geometry = _ => refresh();
                Host.RegisterCallback(geometry);
            }

            public VisualElement Host { get; }
            public List<Entry> Entries { get; }
            public VisualElement? ApplicationReturn { get; set; }
            public VisualElement? PendingRestore { get; set; }
            public Entry? ActiveModal { get; set; }

            public void BindPanel()
            {
                VisualElement? next = Host.panel?.visualTree;
                if (next == panelRoot)
                    return;
                UnbindPanel();
                panelRoot = next;
                panelRoot?.RegisterCallback(focusIn, TrickleDown.TrickleDown);
                panelRoot?.RegisterCallback(keyDown);
            }

            public void Dispose()
            {
                UnbindPanel();
                Host.UnregisterCallback(geometry);
            }

            private void UnbindPanel()
            {
                panelRoot?.UnregisterCallback(focusIn, TrickleDown.TrickleDown);
                panelRoot?.UnregisterCallback(keyDown);
                panelRoot = null;
            }
        }

        private sealed class Entry : IDisposable
        {
            private readonly SystemAction refresh;
            private readonly System.Action<float> scrollChanged;
            private readonly List<ScrollView> scrollViews = new();
            private VisualElement? anchor;

            public Entry(
                VisualElement wrapper,
                Surface surface,
                OverlayPlacement placement,
                int ordinal,
                SystemAction refresh
            )
            {
                Wrapper = wrapper;
                Surface = surface;
                Placement = placement;
                SourceOrdinal = ordinal;
                this.refresh = refresh;
                scrollChanged = _ => refresh();
                wrapper.RegisterCallback<GeometryChangedEvent>(OnWrapperGeometry);
            }

            public VisualElement Wrapper { get; }
            public Surface Surface { get; }
            public OverlayPlacement Placement { get; set; }
            public int SourceOrdinal { get; set; }
            public bool Waiting { get; set; }
            public bool WrapperCurrent { get; set; }
            public bool FocusActivated { get; set; }
            public bool HostFill { get; set; }
            public VisualElement? LastFocused { get; set; }

            public void RebindAnchor(Func<ObjectId, VisualElement?> resolve)
            {
                VisualElement? next = Placement is OverlayPlacement.Popover popover
                    ? resolve(popover.Anchor)
                    : null;
                if (next == anchor)
                    return;
                anchor?.UnregisterCallback<GeometryChangedEvent>(OnAnchorGeometry);
                UnbindScrollViews();
                anchor = next;
                anchor?.RegisterCallback<GeometryChangedEvent>(OnAnchorGeometry);
                BindScrollViews();
            }

            public void Dispose()
            {
                Wrapper.UnregisterCallback<GeometryChangedEvent>(OnWrapperGeometry);
                anchor?.UnregisterCallback<GeometryChangedEvent>(OnAnchorGeometry);
                UnbindScrollViews();
            }

            private void OnWrapperGeometry(GeometryChangedEvent _)
            {
                WrapperCurrent = true;
                refresh();
            }

            private void OnAnchorGeometry(GeometryChangedEvent _) => refresh();

            private void BindScrollViews()
            {
                for (
                    VisualElement? current = anchor;
                    current is not null;
                    current = current.hierarchy.parent
                )
                {
                    if (current is not ScrollView scroll)
                        continue;
                    scrollViews.Add(scroll);
                    scroll.horizontalScroller.valueChanged += scrollChanged;
                    scroll.verticalScroller.valueChanged += scrollChanged;
                }
            }

            private void UnbindScrollViews()
            {
                foreach (ScrollView scroll in scrollViews)
                {
                    scroll.horizontalScroller.valueChanged -= scrollChanged;
                    scroll.verticalScroller.valueChanged -= scrollChanged;
                }
                scrollViews.Clear();
            }
        }
    }
}
