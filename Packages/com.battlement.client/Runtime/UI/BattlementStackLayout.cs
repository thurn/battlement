#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementStackLayout
    {
        private const int MaximumUnstablePasses = 4;

        private readonly BattlementLayoutContainer owner;
        private readonly BattlementLayoutContainerAdapter adapter;
        private UiAlign alignItems = UiAlign.Stretch;
        private UiAlign justifyItems = UiAlign.Stretch;
        private StackSignature? signature;
        private int unstablePasses;
        private bool diagnosticIssued;

        public BattlementStackLayout(
            BattlementLayoutContainer owner,
            BattlementLayoutContainerAdapter adapter
        )
        {
            this.owner = owner;
            this.adapter = adapter;
            owner.RegisterCallback<GeometryChangedEvent>(_ => Refresh());
        }

        public int DiagnosticCount { get; private set; }

        public int PassCount { get; private set; }

        public void Apply(UiElement.Stack value)
        {
            alignItems = Resolve(value.AlignItems, alignItems, UiAlign.Stretch);
            justifyItems = Resolve(value.JustifyItems, justifyItems, UiAlign.Stretch);
            Invalidate();
        }

        public void Invalidate()
        {
            unstablePasses = 0;
            diagnosticIssued = false;
            Refresh();
        }

        public void Refresh()
        {
            try
            {
                VisualElement[] logical = adapter.LogicalChildren.ToArray();
                StackItem[] allItems = logical.Select(BattlementStackItems.Get).ToArray();
                adapter.Present(
                    logical
                        .Select((child, index) => new OrderedChild(child, allItems[index], index))
                        .OrderBy(value => value.Item.Order)
                        .ThenBy(value => value.Index)
                        .Select(value => value.Child)
                        .ToArray()
                );

                for (int index = 0; index < logical.Length; index++)
                    adapter.SlotFor(logical[index]).style.display =
                        logical[index].style.display.value == DisplayStyle.None
                            ? DisplayStyle.None
                            : DisplayStyle.Flex;

                VisualElement[] children = logical
                    .Where(child => child.style.display.value != DisplayStyle.None)
                    .ToArray();
                StackItem[] items = children.Select(BattlementStackItems.Get).ToArray();
                float? boundedWidth = Available(owner, horizontal: true);
                float width = boundedWidth ?? IntrinsicWidth(children, items);
                float? boundedHeight = Available(owner, horizontal: false);
                float height = boundedHeight ?? IntrinsicHeight(children, items, width);
                UnityEngine.Rect[] placements = children
                    .Select((child, index) => Place(child, items[index], width, height))
                    .ToArray();
                Apply(children, items, placements, width, height, boundedWidth, boundedHeight);
            }
            catch (Exception exception)
                when (exception is InvalidOperationException or OverflowException)
            {
                EmitDiagnostic();
            }
        }

        private void Apply(
            IReadOnlyList<VisualElement> children,
            IReadOnlyList<StackItem> items,
            IReadOnlyList<UnityEngine.Rect> placements,
            float width,
            float height,
            float? boundedWidth,
            float? boundedHeight
        )
        {
            var next = new StackSignature(
                children,
                items,
                placements,
                width,
                height,
                alignItems,
                justifyItems
            );
            if (signature is not null && signature.Equals(next))
            {
                unstablePasses = 0;
                return;
            }
            if (unstablePasses >= MaximumUnstablePasses)
            {
                EmitDiagnostic();
                return;
            }

            VisualElement measurement = adapter.Measurement!;
            measurement.style.width = boundedWidth.HasValue ? StyleKeyword.Auto : width;
            measurement.style.height = boundedHeight.HasValue ? StyleKeyword.Auto : height;
            for (int index = 0; index < children.Count; index++)
            {
                BattlementLayoutSlot slot = adapter.SlotFor(children[index]);
                UnityEngine.Rect placement = placements[index];
                slot.style.position = Position.Absolute;
                slot.style.left = placement.x;
                slot.style.top = placement.y;
                slot.style.width = placement.width;
                slot.style.height = placement.height;
            }
            signature = next;
            unstablePasses++;
            PassCount++;
        }

        private UnityEngine.Rect Place(
            VisualElement child,
            StackItem item,
            float width,
            float height
        )
        {
            float left = item.Left ?? 0;
            float right = Math.Max(left, width - (item.Right ?? 0));
            float top = item.Top ?? 0;
            float bottom = Math.Max(top, height - (item.Bottom ?? 0));
            left += Margin(child.style.marginLeft);
            right = Math.Max(left, right - Margin(child.style.marginRight));
            top += Margin(child.style.marginTop);
            bottom = Math.Max(top, bottom - Margin(child.style.marginBottom));
            float availableWidth = right - left;
            float preferredWidth = PreferredWidth(child);
            UiAlign horizontal = item.JustifySelf == UiAlign.Auto ? justifyItems : item.JustifySelf;
            (float placedLeft, float placedWidth) = Align(
                left,
                availableWidth,
                preferredWidth,
                horizontal,
                HasWidth(child),
                Minimum(child.style.minWidth),
                Maximum(child.style.maxWidth)
            );
            left = placedLeft;
            float preferredHeight = PreferredHeight(child, placedWidth);
            UiAlign vertical = item.AlignSelf == UiAlign.Auto ? alignItems : item.AlignSelf;
            (float placedTop, float placedHeight) = Align(
                top,
                bottom - top,
                preferredHeight,
                vertical,
                HasHeight(child),
                Minimum(child.style.minHeight),
                Maximum(child.style.maxHeight)
            );
            top = placedTop;
            return new UnityEngine.Rect(left, top, placedWidth, placedHeight);
        }

        private static (float Position, float Size) Align(
            float position,
            float available,
            float preferred,
            UiAlign align,
            bool authoredSize,
            float minimum,
            float maximum
        )
        {
            float size = align == UiAlign.Stretch && !authoredSize ? available : preferred;
            size = Math.Min(maximum, Math.Max(minimum, size));
            return align switch
            {
                UiAlign.Center => (position + (available - size) / 2, size),
                UiAlign.FlexEnd => (position + available - size, size),
                _ => (position, size),
            };
        }

        private static float IntrinsicWidth(
            IReadOnlyList<VisualElement> children,
            IReadOnlyList<StackItem> items
        ) =>
            children
                .Select(
                    (child, index) =>
                        items[index].ContributesToSize
                            ? PreferredWidth(child)
                                + Margin(child.style.marginLeft)
                                + Margin(child.style.marginRight)
                                + (items[index].Left ?? 0)
                                + (items[index].Right ?? 0)
                            : 0
                )
                .DefaultIfEmpty(0)
                .Max();

        private static float IntrinsicHeight(
            IReadOnlyList<VisualElement> children,
            IReadOnlyList<StackItem> items,
            float width
        ) =>
            children
                .Select(
                    (child, index) =>
                        items[index].ContributesToSize
                            ? PreferredHeight(
                                child,
                                Math.Max(
                                    0,
                                    width
                                        - (items[index].Left ?? 0)
                                        - (items[index].Right ?? 0)
                                        - Margin(child.style.marginLeft)
                                        - Margin(child.style.marginRight)
                                )
                            )
                                + Margin(child.style.marginTop)
                                + Margin(child.style.marginBottom)
                                + (items[index].Top ?? 0)
                                + (items[index].Bottom ?? 0)
                            : 0
                )
                .DefaultIfEmpty(0)
                .Max();

        private static float PreferredWidth(VisualElement child)
        {
            float value =
                AuthoredPixels(child.style.width)
                ?? MeasuredTextWidth(child)
                ?? FinitePositive(child.layout.width)
                ?? FinitePositive(child.resolvedStyle.width)
                ?? 0;
            return Math.Min(
                Maximum(child.style.maxWidth),
                Math.Max(Minimum(child.style.minWidth), value)
            );
        }

        private static float PreferredHeight(VisualElement child, float width)
        {
            float value;
            if (AuthoredPixels(child.style.height) is float authored)
                value = authored;
            else if (child is TextElement text && !string.IsNullOrEmpty(text.text))
                value = text.MeasureTextSize(
                    text.text,
                    Math.Max(0, width),
                    VisualElement.MeasureMode.Exactly,
                    0,
                    VisualElement.MeasureMode.Undefined
                ).y;
            else
                value =
                    FinitePositive(child.layout.height)
                    ?? FinitePositive(child.resolvedStyle.height)
                    ?? 0;
            return Math.Min(
                Maximum(child.style.maxHeight),
                Math.Max(Minimum(child.style.minHeight), Math.Max(0, value))
            );
        }

        private static float? MeasuredTextWidth(VisualElement child)
        {
            if (child is not TextElement text || string.IsNullOrEmpty(text.text))
                return null;
            return FinitePositive(
                text.MeasureTextSize(
                    text.text,
                    0,
                    VisualElement.MeasureMode.Undefined,
                    0,
                    VisualElement.MeasureMode.Undefined
                ).x
            );
        }

        private static float? Available(VisualElement value, bool horizontal)
        {
            float? authored = AuthoredPixels(horizontal ? value.style.width : value.style.height);
            if (authored.HasValue)
                return authored;
            float resolved = horizontal ? value.contentRect.width : value.contentRect.height;
            return value.panel is null ? null : FinitePositive(resolved);
        }

        private static bool HasWidth(VisualElement value) =>
            AuthoredPixels(value.style.width).HasValue;

        private static bool HasHeight(VisualElement value) =>
            AuthoredPixels(value.style.height).HasValue;

        private static float Minimum(StyleLength value) => AuthoredPixels(value) ?? 0;

        private static float Maximum(StyleLength value) => AuthoredPixels(value) ?? float.MaxValue;

        private static float? AuthoredPixels(StyleLength value) =>
            value.keyword == StyleKeyword.Undefined
            && value.value.unit == LengthUnit.Pixel
            && float.IsFinite(value.value.value)
                ? value.value.value
                : null;

        private static float? FinitePositive(float value) =>
            float.IsFinite(value) && value > 0 ? value : null;

        private static float Margin(StyleLength value) => AuthoredPixels(value) ?? 0;

        private void EmitDiagnostic()
        {
            if (diagnosticIssued)
                return;
            diagnosticIssued = true;
            DiagnosticCount++;
        }

        private static T Resolve<T>(Prop<T> value, T current, T reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private readonly struct OrderedChild
        {
            public OrderedChild(VisualElement child, StackItem item, int index)
            {
                Child = child;
                Item = item;
                Index = index;
            }

            public VisualElement Child { get; }

            public StackItem Item { get; }

            public int Index { get; }
        }

        private sealed class StackSignature : IEquatable<StackSignature>
        {
            private readonly UiAlign alignItems;
            private readonly VisualElement[] children;
            private readonly float height;
            private readonly StackItem[] items;
            private readonly UiAlign justifyItems;
            private readonly UnityEngine.Rect[] placements;
            private readonly float width;

            public StackSignature(
                IReadOnlyList<VisualElement> children,
                IReadOnlyList<StackItem> items,
                IReadOnlyList<UnityEngine.Rect> placements,
                float width,
                float height,
                UiAlign alignItems,
                UiAlign justifyItems
            )
            {
                this.children = children.ToArray();
                this.items = items.ToArray();
                this.placements = placements.ToArray();
                this.width = width;
                this.height = height;
                this.alignItems = alignItems;
                this.justifyItems = justifyItems;
            }

            public bool Equals(StackSignature? other) =>
                other is not null
                && width.Equals(other.width)
                && height.Equals(other.height)
                && alignItems == other.alignItems
                && justifyItems == other.justifyItems
                && children.SequenceEqual(other.children)
                && items.SequenceEqual(other.items)
                && placements.SequenceEqual(other.placements);
        }
    }
}
