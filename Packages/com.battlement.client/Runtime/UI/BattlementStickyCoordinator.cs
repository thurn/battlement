#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementStickyCoordinator
    {
        private readonly Dictionary<VisualElement, Entry> entries = new();
        private readonly Dictionary<ScrollView, Surface> surfaces = new();

        public int UpdateCount { get; private set; }

        public void Apply(VisualElement target, Prop<Sticky> value, int sourceOrdinal)
        {
            BattlementStickyItems.Apply(target, value);
            if (value.IsReset)
            {
                Detach(target, restore: true);
                return;
            }
            if (!BattlementStickyItems.HasAuthored(target))
                return;
            if (entries.TryGetValue(target, out Entry existing))
            {
                existing.Descriptor = BattlementStickyItems.Get(target);
                existing.SourceOrdinal = sourceOrdinal;
                existing.Surface.RefreshOrder();
                return;
            }
            Attach(target, sourceOrdinal);
        }

        public void Refresh(VisualElement target)
        {
            if (entries.TryGetValue(target, out Entry entry))
                entry.Surface.RefreshAll();
        }

        public void RefreshOrdinals(Func<VisualElement, int> ordinal)
        {
            foreach (Entry entry in entries.Values)
                entry.SourceOrdinal = ordinal(entry.Host);
            foreach (Surface surface in surfaces.Values)
                surface.RefreshOrder();
        }

        public void PrepareHierarchyChange(VisualElement target) => Detach(target, restore: true);

        public void Remove(VisualElement target) => Detach(target, restore: false);

        public void Clear()
        {
            foreach (VisualElement target in entries.Keys.ToArray())
                Detach(target, restore: false);
        }

        public bool TryGetPlaceholder(VisualElement target, out BattlementLayoutSlot? slot)
        {
            bool found = entries.TryGetValue(target, out Entry entry);
            slot = found ? entry.Slot : null;
            return found;
        }

        public bool TryGetPresentation(VisualElement target, out VisualElement? presentation)
        {
            bool found = entries.TryGetValue(target, out Entry entry);
            presentation = found ? entry.Presentation : null;
            return found;
        }

        internal static float ResolveLeading(
            float normalStart,
            float viewportStart,
            float containingEnd,
            float size,
            float inset
        ) => Math.Min(Math.Max(normalStart, viewportStart + inset), containingEnd - size);

        internal static float ResolveTrailing(
            float normalEnd,
            float viewportEnd,
            float containingStart,
            float size,
            float inset
        ) => Math.Max(Math.Min(normalEnd, viewportEnd - inset), containingStart + size) - size;

        private void Attach(VisualElement target, int sourceOrdinal)
        {
            VisualElement physicalParent =
                target.hierarchy.parent
                ?? throw new InvalidOperationException(
                    "Sticky requires attachment beneath a physical ScrollView."
                );
            ScrollView scroll =
                FindScrollView(physicalParent)
                ?? throw new InvalidOperationException(
                    "Sticky requires attachment beneath a physical ScrollView."
                );
            bool ownsSlot = physicalParent is not BattlementLayoutSlot;
            BattlementLayoutSlot slot;
            if (physicalParent is BattlementLayoutSlot existing)
                slot = existing;
            else
            {
                int index = physicalParent.hierarchy.IndexOf(target);
                target.RemoveFromHierarchy();
                slot = new BattlementLayoutSlot(target, physicalParent);
                physicalParent.hierarchy.Insert(index, slot);
            }

            Surface surface = GetSurface(scroll);
            var entry = new Entry(
                target,
                slot,
                ownsSlot,
                BattlementStickyItems.Get(target),
                sourceOrdinal,
                surface,
                surface.RefreshAll
            );
            slot.DetachHost();
            entries.Add(target, entry);
            surface.Add(entry);
        }

        private void Detach(VisualElement target, bool restore)
        {
            if (!entries.Remove(target, out Entry entry))
                return;
            entry.Surface.Remove(entry);
            entry.Dispose();
            target.RemoveFromHierarchy();
            if (restore)
                Restore(entry);
            else if (entry.OwnsSlot)
                entry.Slot.RemoveFromHierarchy();
            if (entry.Surface.Count != 0)
                return;
            surfaces.Remove(entry.Surface.Scroll);
            entry.Surface.Dispose();
        }

        private static void Restore(Entry entry)
        {
            if (!entry.OwnsSlot)
            {
                entry.Slot.AttachHost();
                return;
            }
            VisualElement? parent = entry.Slot.hierarchy.parent;
            if (parent is null)
                return;
            int index = parent.hierarchy.IndexOf(entry.Slot);
            entry.Slot.RemoveFromHierarchy();
            parent.hierarchy.Insert(index, entry.Host);
        }

        private Surface GetSurface(ScrollView scroll)
        {
            if (surfaces.TryGetValue(scroll, out Surface value))
                return value;
            value = new Surface(scroll, () => UpdateCount++);
            surfaces.Add(scroll, value);
            return value;
        }

        private static ScrollView? FindScrollView(VisualElement value)
        {
            for (VisualElement? current = value; current is not null; current = current.parent)
            {
                if (current is ScrollView scroll)
                    return scroll;
            }
            return null;
        }

        private sealed class Surface : IDisposable
        {
            private readonly List<Entry> entries = new();
            private readonly System.Action invalidated;
            private readonly System.Action<float> scrollChanged;
            private readonly EventCallback<GeometryChangedEvent> geometryChanged;

            public Surface(ScrollView scroll, System.Action invalidated)
            {
                Scroll = scroll;
                this.invalidated = invalidated;
                Root = new VisualElement
                {
                    focusable = false,
                    pickingMode = PickingMode.Ignore,
                    tabIndex = -1,
                };
                Root.style.position = Position.Absolute;
                Root.style.left = 0;
                Root.style.right = 0;
                Root.style.top = 0;
                Root.style.bottom = 0;
                Root.style.overflow = Overflow.Hidden;
                scroll.contentViewport.hierarchy.Add(Root);
                scrollChanged = _ => RefreshAll();
                geometryChanged = _ => RefreshAll();
                scroll.horizontalScroller.valueChanged += scrollChanged;
                scroll.verticalScroller.valueChanged += scrollChanged;
                scroll.contentViewport.RegisterCallback(geometryChanged);
            }

            public int Count => entries.Count;

            public VisualElement Root { get; }

            public ScrollView Scroll { get; }

            public void Add(Entry entry)
            {
                entries.Add(entry);
                Root.hierarchy.Add(entry.Presentation);
                entry.Register();
                RefreshOrder();
            }

            public void Remove(Entry entry)
            {
                entries.Remove(entry);
                entry.Presentation.RemoveFromHierarchy();
                RefreshAll();
            }

            public void RefreshAll()
            {
                foreach (Entry entry in entries)
                    entry.Refresh();
                invalidated();
            }

            public void RefreshOrder()
            {
                entries.Sort(Compare);
                for (int index = 0; index < entries.Count; index++)
                {
                    Entry entry = entries[index];
                    if (index == 0)
                        entry.Presentation.SendToBack();
                    else
                        entry.Presentation.PlaceInFront(entries[index - 1].Presentation);
                }
                RefreshAll();
            }

            public void Dispose()
            {
                Scroll.horizontalScroller.valueChanged -= scrollChanged;
                Scroll.verticalScroller.valueChanged -= scrollChanged;
                Scroll.contentViewport.UnregisterCallback(geometryChanged);
                Root.RemoveFromHierarchy();
            }

            private static int Compare(Entry left, Entry right)
            {
                int order = left.Descriptor.Order.CompareTo(right.Descriptor.Order);
                return order != 0 ? order : left.SourceOrdinal.CompareTo(right.SourceOrdinal);
            }
        }

        private sealed class Entry : IDisposable
        {
            private readonly System.Action invalidated;
            private readonly EventCallback<GeometryChangedEvent> geometryChanged;
            private UnityEngine.Rect lastNormal;

            public Entry(
                VisualElement host,
                BattlementLayoutSlot slot,
                bool ownsSlot,
                Sticky descriptor,
                int sourceOrdinal,
                Surface surface,
                System.Action invalidated
            )
            {
                Host = host;
                Slot = slot;
                OwnsSlot = ownsSlot;
                Descriptor = descriptor;
                SourceOrdinal = sourceOrdinal;
                Surface = surface;
                this.invalidated = invalidated;
                lastNormal = slot.worldBound;
                Presentation = new VisualElement
                {
                    focusable = false,
                    pickingMode = PickingMode.Ignore,
                    tabIndex = -1,
                };
                Presentation.style.position = Position.Absolute;
                Presentation.hierarchy.Add(host);
                geometryChanged = _ => invalidated();
            }

            public Sticky Descriptor { get; set; }

            public VisualElement Host { get; }

            public bool OwnsSlot { get; }

            public VisualElement Presentation { get; }

            public int SourceOrdinal { get; set; }

            public BattlementLayoutSlot Slot { get; }

            public Surface Surface { get; }

            public void Register()
            {
                Host.RegisterCallback(geometryChanged);
                Slot.RegisterCallback(geometryChanged);
                Slot.ContainingBlock.RegisterCallback(geometryChanged);
            }

            public void Refresh()
            {
                float width = Finite(Host.layout.width, lastNormal.width);
                float height = Finite(Host.layout.height, lastNormal.height);
                if (
                    OwnsSlot
                    || Slot.ContainingBlock
                        is BattlementLayoutContainer { Kind: BattlementLayoutContainerKind.Flex }
                )
                {
                    Slot.style.width = width;
                    Slot.style.height = height;
                }
                UnityEngine.Rect normal = Slot.worldBound;
                if (Finite(normal.width, 0) > 0 || Finite(normal.height, 0) > 0)
                    lastNormal = normal;
                else
                    normal = lastNormal;
                UnityEngine.Rect viewport = Surface.Scroll.contentViewport.worldBound;
                UnityEngine.Rect containing = ContentWorldBound(Slot.ContainingBlock);
                float left =
                    Descriptor.Left is float leftInset
                        ? ResolveLeading(
                            normal.xMin,
                            viewport.xMin,
                            containing.xMax,
                            width,
                            leftInset
                        )
                    : Descriptor.Right is float rightInset
                        ? ResolveTrailing(
                            normal.xMax,
                            viewport.xMax,
                            containing.xMin,
                            width,
                            rightInset
                        )
                    : normal.xMin;
                float top =
                    Descriptor.Top is float topInset
                        ? ResolveLeading(
                            normal.yMin,
                            viewport.yMin,
                            containing.yMax,
                            height,
                            topInset
                        )
                    : Descriptor.Bottom is float bottomInset
                        ? ResolveTrailing(
                            normal.yMax,
                            viewport.yMax,
                            containing.yMin,
                            height,
                            bottomInset
                        )
                    : normal.yMin;
                UnityEngine.Rect surface = Surface.Root.worldBound;
                Presentation.style.left = left - surface.xMin;
                Presentation.style.top = top - surface.yMin;
                Presentation.style.width = width;
                Presentation.style.height = height;
            }

            public void Dispose()
            {
                Host.UnregisterCallback(geometryChanged);
                Slot.UnregisterCallback(geometryChanged);
                Slot.ContainingBlock.UnregisterCallback(geometryChanged);
            }

            private static UnityEngine.Rect ContentWorldBound(VisualElement value)
            {
                Vector2 origin = value.LocalToWorld(value.contentRect.position);
                return new UnityEngine.Rect(origin, value.contentRect.size);
            }

            private static float Finite(float value, float fallback) =>
                float.IsFinite(value) && value >= 0 ? value : fallback;
        }
    }
}
