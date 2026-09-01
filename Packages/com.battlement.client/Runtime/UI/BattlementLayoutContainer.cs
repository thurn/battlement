#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal enum BattlementLayoutContainerKind
    {
        Flex,
        Grid,
        Stack,
    }

    internal readonly struct BattlementPortalSourceOrdinal
        : IComparable<BattlementPortalSourceOrdinal>,
            IEquatable<BattlementPortalSourceOrdinal>
    {
        public BattlementPortalSourceOrdinal(uint root, uint portal)
        {
            Root = root;
            Portal = portal;
        }

        public uint Root { get; }

        public uint Portal { get; }

        public int CompareTo(BattlementPortalSourceOrdinal other)
        {
            int root = Root.CompareTo(other.Root);
            return root != 0 ? root : Portal.CompareTo(other.Portal);
        }

        public bool Equals(BattlementPortalSourceOrdinal other) =>
            Root == other.Root && Portal == other.Portal;

        public override bool Equals(object? value) =>
            value is BattlementPortalSourceOrdinal other && Equals(other);

        public override int GetHashCode() => HashCode.Combine(Root, Portal);
    }

    internal sealed class BattlementLayoutContainer : VisualElement
    {
        private readonly BattlementFlexLayout? flexLayout;
        private readonly BattlementGridLayout? gridLayout;
        private bool layoutDirty = true;

        public BattlementLayoutContainer(BattlementLayoutContainerKind kind)
        {
            Kind = kind;
            Adapter = new BattlementLayoutContainerAdapter(
                this,
                kind is BattlementLayoutContainerKind.Grid or BattlementLayoutContainerKind.Stack,
                MarkLayoutDirty
            );
            if (kind == BattlementLayoutContainerKind.Flex)
                flexLayout = new BattlementFlexLayout(this, Adapter);
            if (kind == BattlementLayoutContainerKind.Grid)
                gridLayout = new BattlementGridLayout(this, Adapter);
        }

        public BattlementLayoutContainerKind Kind { get; }

        public BattlementLayoutContainerAdapter Adapter { get; }

        public BattlementFlexLayout? FlexLayout => flexLayout;

        public BattlementGridLayout? GridLayout => gridLayout;

        public void ApplyFlex(UiElement.Flex value)
        {
            if (flexLayout is null)
                throw new InvalidOperationException(
                    "Only a Flex container accepts Flex properties."
                );
            flexLayout.Apply(value);
            layoutDirty = true;
        }

        public void ApplyGrid(UiElement.Grid value)
        {
            if (gridLayout is null)
                throw new InvalidOperationException(
                    "Only a Grid container accepts Grid properties."
                );
            gridLayout.Apply(value);
            layoutDirty = true;
        }

        public bool TakeLayoutDirty()
        {
            bool dirty = layoutDirty;
            layoutDirty = false;
            return dirty;
        }

        private void MarkLayoutDirty()
        {
            layoutDirty = true;
            flexLayout?.Refresh();
            gridLayout?.Invalidate();
        }
    }

    internal sealed class BattlementLayoutContainerAdapter
    {
        private readonly VisualElement owner;
        private readonly VisualElement? measurement;
        private readonly System.Action markDirty;
        private readonly List<VisualElement> directChildren = new();
        private readonly List<PortalChild> portalChildren = new();
        private readonly Dictionary<VisualElement, BattlementLayoutSlot> slots = new();

        public BattlementLayoutContainerAdapter(
            VisualElement owner,
            bool usesMeasurement,
            System.Action markDirty
        )
        {
            this.owner = owner;
            this.markDirty = markDirty;
            if (usesMeasurement)
            {
                measurement = PrivateElement();
                owner.hierarchy.Add(measurement);
            }
        }

        public IReadOnlyList<VisualElement> LogicalChildren =>
            directChildren.Concat(OrderedPortals().Select(value => value.Host)).ToArray();

        public VisualElement? Measurement => measurement;

        public void Insert(VisualElement child, int index)
        {
            if (index < 0 || index > directChildren.Count)
                throw new ArgumentOutOfRangeException(nameof(index));
            RequireUnattached(child);
            directChildren.Insert(index, child);
            slots.Add(child, new BattlementLayoutSlot(child));
            PresentLogicalOrder();
            markDirty();
        }

        public void AttachPortal(VisualElement child, BattlementPortalSourceOrdinal source)
        {
            RequireUnattached(child);
            portalChildren.Add(new PortalChild(child, source));
            slots.Add(child, new BattlementLayoutSlot(child));
            PresentLogicalOrder();
            markDirty();
        }

        public void Reindex(VisualElement child, int index)
        {
            int previous = directChildren.IndexOf(child);
            if (previous < 0)
                throw new InvalidOperationException(
                    "Only a direct logical child can be reindexed."
                );
            if (index < 0 || index >= directChildren.Count)
                throw new ArgumentOutOfRangeException(nameof(index));
            directChildren.RemoveAt(previous);
            directChildren.Insert(index, child);
            PresentLogicalOrder();
            markDirty();
        }

        public void Detach(VisualElement child)
        {
            bool removed = directChildren.Remove(child);
            int portal = portalChildren.FindIndex(value => value.Host == child);
            if (portal >= 0)
            {
                portalChildren.RemoveAt(portal);
                removed = true;
            }
            if (!removed || !slots.Remove(child, out BattlementLayoutSlot slot))
                throw new InvalidOperationException("The child is not attached to this adapter.");
            slot.DetachHost();
            slot.RemoveFromHierarchy();
            markDirty();
        }

        public void Present(IReadOnlyList<VisualElement> children)
        {
            VisualElement[] logical = LogicalChildren.ToArray();
            if (children.Count != logical.Length || children.Distinct().Count() != children.Count)
                throw new ArgumentException("Presentation order must contain every child once.");
            if (children.Any(child => !slots.ContainsKey(child)))
                throw new ArgumentException("Presentation order contains an unknown child.");
            PresentSlots(children);
        }

        public bool TryGetSlot(VisualElement child, out VisualElement? slot)
        {
            bool found = slots.TryGetValue(child, out BattlementLayoutSlot value);
            slot = found ? value : null;
            return found;
        }

        public BattlementLayoutSlot SlotFor(VisualElement child) =>
            slots.TryGetValue(child, out BattlementLayoutSlot value)
                ? value
                : throw new InvalidOperationException("The child is not attached to this adapter.");

        public void Clear()
        {
            foreach (VisualElement child in LogicalChildren)
            {
                BattlementLayoutSlot slot = slots[child];
                slot.DetachHost();
                slot.RemoveFromHierarchy();
            }
            directChildren.Clear();
            portalChildren.Clear();
            slots.Clear();
            measurement?.RemoveFromHierarchy();
            markDirty();
        }

        private void RequireUnattached(VisualElement child)
        {
            if (slots.ContainsKey(child))
                throw new InvalidOperationException(
                    "The child is already attached to this adapter."
                );
        }

        private IEnumerable<PortalChild> OrderedPortals() =>
            portalChildren.OrderBy(value => value.Source);

        private void PresentLogicalOrder() => PresentSlots(LogicalChildren);

        private void PresentSlots(IReadOnlyList<VisualElement> children)
        {
            foreach (BattlementLayoutSlot slot in slots.Values)
                slot.RemoveFromHierarchy();
            int offset = measurement is null ? 0 : 1;
            for (int index = 0; index < children.Count; index++)
                owner.hierarchy.Insert(offset + index, slots[children[index]]);
        }

        private static VisualElement PrivateElement() =>
            new()
            {
                focusable = false,
                pickingMode = PickingMode.Ignore,
                tabIndex = -1,
            };

        private sealed record PortalChild(VisualElement Host, BattlementPortalSourceOrdinal Source);
    }

    internal sealed class BattlementLayoutSlot : VisualElement
    {
        public BattlementLayoutSlot(VisualElement host)
        {
            Host = host;
            focusable = false;
            pickingMode = PickingMode.Ignore;
            tabIndex = -1;
            hierarchy.Add(host);
        }

        public VisualElement Host { get; }

        public void DetachHost()
        {
            if (Host.parent == this)
                Host.RemoveFromHierarchy();
        }
    }
}
