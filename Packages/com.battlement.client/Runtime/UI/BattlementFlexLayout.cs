#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;
using UnityAlign = UnityEngine.UIElements.Align;
using UnityFlexDirection = UnityEngine.UIElements.FlexDirection;
using UnityFlexWrap = UnityEngine.UIElements.Wrap;
using UnityJustify = UnityEngine.UIElements.Justify;

namespace Battlement.UI
{
    internal sealed class BattlementFlexLayout
    {
        private readonly BattlementLayoutContainer owner;
        private readonly BattlementLayoutContainerAdapter adapter;
        private readonly VisualElement flowBand;
        private UiFlexDirection direction = UiFlexDirection.Column;
        private UiFlexWrap wrap = UiFlexWrap.NoWrap;
        private UiAlign alignItems = UiAlign.Stretch;
        private UiJustify justifyContent = UiJustify.FlexStart;
        private float rowGap;
        private float columnGap;
        private LayoutSignature? signature;

        public BattlementFlexLayout(
            BattlementLayoutContainer owner,
            BattlementLayoutContainerAdapter adapter
        )
        {
            this.owner = owner;
            this.adapter = adapter;
            flowBand = new VisualElement
            {
                focusable = false,
                pickingMode = PickingMode.Ignore,
                tabIndex = -1,
            };
            flowBand.style.flexGrow = 1;
            owner.hierarchy.Add(flowBand);
            ApplyBandStyle();
        }

        public VisualElement FlowBand => flowBand;

        public int PassCount { get; private set; }

        public void Apply(UiElement.Flex value)
        {
            direction = Resolve(value.Direction, direction, UiFlexDirection.Column);
            wrap = Resolve(value.Wrap, wrap, UiFlexWrap.NoWrap);
            alignItems = Resolve(value.AlignItems, alignItems, UiAlign.Stretch);
            justifyContent = Resolve(value.JustifyContent, justifyContent, UiJustify.FlexStart);
            rowGap = Resolve(value.RowGap, rowGap, 0);
            columnGap = Resolve(value.ColumnGap, columnGap, 0);
            ApplyBandStyle();
            Refresh();
        }

        public void Refresh()
        {
            IReadOnlyList<VisualElement> logical = adapter.LogicalChildren;
            VisualElement[] inFlow = logical.Where(IsInFlow).ToArray();
            VisualElement[] absolute = logical.Where(child => !IsInFlow(child)).ToArray();
            LayoutSignature next = new(
                direction,
                wrap,
                alignItems,
                justifyContent,
                rowGap,
                columnGap,
                inFlow,
                absolute
            );
            if (signature is not null && signature.Equals(next))
                return;

            flowBand.Clear();
            foreach (VisualElement child in inFlow)
            {
                BattlementLayoutSlot slot = adapter.SlotFor(child);
                ConfigureFlowSlot(slot);
                flowBand.hierarchy.Add(slot);
            }
            foreach (VisualElement child in absolute)
            {
                BattlementLayoutSlot slot = adapter.SlotFor(child);
                ConfigureAbsoluteSlot(slot, child);
                owner.hierarchy.Add(slot);
            }
            ApplyOuterCompensation(inFlow.Any(IsDisplayed));
            signature = next;
            PassCount++;
        }

        private void ApplyBandStyle()
        {
            flowBand.style.flexDirection = ToUnity(direction);
            flowBand.style.flexWrap = ToUnity(wrap);
            flowBand.style.alignItems = ToUnity(alignItems);
            flowBand.style.justifyContent = ToUnity(justifyContent);
        }

        private void ConfigureFlowSlot(BattlementLayoutSlot slot)
        {
            slot.style.position = Position.Relative;
            slot.style.left = StyleKeyword.Null;
            slot.style.right = StyleKeyword.Null;
            slot.style.top = StyleKeyword.Null;
            slot.style.bottom = StyleKeyword.Null;
            slot.style.marginLeft = columnGap / 2;
            slot.style.marginRight = columnGap / 2;
            slot.style.marginTop = rowGap / 2;
            slot.style.marginBottom = rowGap / 2;
            slot.style.flexGrow = slot.Host.style.flexGrow;
            slot.style.flexShrink = slot.Host.style.flexShrink;
            slot.style.flexBasis = slot.Host.style.flexBasis;
            slot.style.alignSelf = slot.Host.style.alignSelf;
        }

        private static void ConfigureAbsoluteSlot(BattlementLayoutSlot slot, VisualElement child)
        {
            slot.style.position = Position.Absolute;
            slot.style.left = child.style.left;
            slot.style.right = child.style.right;
            slot.style.top = child.style.top;
            slot.style.bottom = child.style.bottom;
            slot.style.width = child.style.width;
            slot.style.height = child.style.height;
            slot.style.minWidth = child.style.minWidth;
            slot.style.minHeight = child.style.minHeight;
            slot.style.maxWidth = child.style.maxWidth;
            slot.style.maxHeight = child.style.maxHeight;
            slot.style.marginLeft = 0;
            slot.style.marginRight = 0;
            slot.style.marginTop = 0;
            slot.style.marginBottom = 0;
        }

        private void ApplyOuterCompensation(bool hasDisplayedInFlow)
        {
            float horizontal = hasDisplayedInFlow ? -columnGap / 2 : 0;
            float vertical = hasDisplayedInFlow ? -rowGap / 2 : 0;
            flowBand.style.marginLeft = horizontal;
            flowBand.style.marginRight = horizontal;
            flowBand.style.marginTop = vertical;
            flowBand.style.marginBottom = vertical;
        }

        private static bool IsInFlow(VisualElement child) =>
            child.style.position.value != Position.Absolute;

        private static bool IsDisplayed(VisualElement child) =>
            child.style.display.value != DisplayStyle.None;

        private static T Resolve<T>(Prop<T> value, T current, T reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private static UnityFlexDirection ToUnity(UiFlexDirection value) =>
            value switch
            {
                UiFlexDirection.Column => UnityFlexDirection.Column,
                UiFlexDirection.ColumnReverse => UnityFlexDirection.ColumnReverse,
                UiFlexDirection.Row => UnityFlexDirection.Row,
                UiFlexDirection.RowReverse => UnityFlexDirection.RowReverse,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private static UnityFlexWrap ToUnity(UiFlexWrap value) =>
            value switch
            {
                UiFlexWrap.NoWrap => UnityFlexWrap.NoWrap,
                UiFlexWrap.Wrap => UnityFlexWrap.Wrap,
                UiFlexWrap.WrapReverse => UnityFlexWrap.WrapReverse,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private static UnityAlign ToUnity(UiAlign value) =>
            value switch
            {
                UiAlign.FlexStart => UnityAlign.FlexStart,
                UiAlign.Center => UnityAlign.Center,
                UiAlign.FlexEnd => UnityAlign.FlexEnd,
                UiAlign.Stretch => UnityAlign.Stretch,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private static UnityJustify ToUnity(UiJustify value) =>
            value switch
            {
                UiJustify.FlexStart => UnityJustify.FlexStart,
                UiJustify.Center => UnityJustify.Center,
                UiJustify.FlexEnd => UnityJustify.FlexEnd,
                UiJustify.SpaceBetween => UnityJustify.SpaceBetween,
                UiJustify.SpaceAround => UnityJustify.SpaceAround,
                UiJustify.SpaceEvenly => UnityJustify.SpaceEvenly,
                _ => throw new ArgumentOutOfRangeException(nameof(value)),
            };

        private sealed class LayoutSignature : IEquatable<LayoutSignature>
        {
            private readonly object[] values;

            public LayoutSignature(
                UiFlexDirection direction,
                UiFlexWrap wrap,
                UiAlign alignItems,
                UiJustify justifyContent,
                float rowGap,
                float columnGap,
                IReadOnlyList<VisualElement> inFlow,
                IReadOnlyList<VisualElement> absolute
            ) =>
                values = new object[]
                {
                    direction,
                    wrap,
                    alignItems,
                    justifyContent,
                    rowGap,
                    columnGap,
                    inFlow.ToArray(),
                    absolute.ToArray(),
                    inFlow.Select(IsDisplayed).ToArray(),
                };

            public bool Equals(LayoutSignature? other) =>
                other is not null && values.SequenceEqual(other.values, ValueComparer.Instance);

            private sealed class ValueComparer : IEqualityComparer<object>
            {
                public static ValueComparer Instance { get; } = new();

                public new bool Equals(object? left, object? right) =>
                    left is Array leftArray && right is Array rightArray
                        ? leftArray.Cast<object>().SequenceEqual(rightArray.Cast<object>())
                        : object.Equals(left, right);

                public int GetHashCode(object value) => value.GetHashCode();
            }
        }
    }
}
