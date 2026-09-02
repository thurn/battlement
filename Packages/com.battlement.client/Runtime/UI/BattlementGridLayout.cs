#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementGridLayout
    {
        private const int MaximumUnstablePasses = 4;

        private readonly BattlementLayoutContainer owner;
        private readonly BattlementLayoutContainerAdapter adapter;
        private IReadOnlyList<GridTrack> columns = Array.Empty<GridTrack>();
        private IReadOnlyList<GridTrack> rows = Array.Empty<GridTrack>();
        private GridTrack autoColumns = new GridTrack.Auto();
        private GridTrack autoRows = new GridTrack.Auto();
        private GridAutoFlow autoFlow = GridAutoFlow.Row;
        private UiAlign alignItems = UiAlign.Stretch;
        private UiAlign justifyItems = UiAlign.Stretch;
        private float rowGap;
        private float columnGap;
        private GridSignature? signature;
        private int unstablePasses;
        private bool diagnosticIssued;

        public BattlementGridLayout(
            BattlementLayoutContainer owner,
            BattlementLayoutContainerAdapter adapter
        )
        {
            this.owner = owner;
            this.adapter = adapter;
            owner.RegisterCallback<GeometryChangedEvent>(_ => Refresh());
        }

        public int DiagnosticCount { get; private set; }

        public int LastMeasuredItemAxes { get; private set; }

        public int PassCount { get; private set; }

        public void Apply(UiElement.Grid value)
        {
            columns = Resolve(value.Columns, columns);
            rows = Resolve(value.Rows, rows);
            autoColumns = Resolve(value.AutoColumns, autoColumns, new GridTrack.Auto());
            autoRows = Resolve(value.AutoRows, autoRows, new GridTrack.Auto());
            autoFlow = Resolve(value.AutoFlow, autoFlow, GridAutoFlow.Row);
            alignItems = Resolve(value.AlignItems, alignItems, UiAlign.Stretch);
            justifyItems = Resolve(value.JustifyItems, justifyItems, UiAlign.Stretch);
            rowGap = Resolve(value.RowGap, rowGap, 0);
            columnGap = Resolve(value.ColumnGap, columnGap, 0);
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
            VisualElement[] children = adapter
                .LogicalChildren.Where(child => child.style.display.value != DisplayStyle.None)
                .ToArray();
            LastMeasuredItemAxes = children.Length * 2;
            try
            {
                GridItem[] items = children.Select(BattlementGridItems.Get).ToArray();
                BattlementGridPlacementResult placement = BattlementGridOccupancy.Place(
                    items,
                    rows.Count,
                    columns.Count,
                    autoFlow
                );
                BattlementGridAxis columnAxis = BattlementGridTrackSizing.Resolve(
                    columns,
                    autoColumns,
                    placement.Columns,
                    columnGap,
                    Available(owner, horizontal: true),
                    children
                        .Select(
                            (child, index) =>
                                new BattlementGridContribution(
                                    placement.Items[index].Column,
                                    placement.Items[index].ColumnSpan,
                                    PreferredOuterWidth(child)
                                )
                        )
                        .ToArray()
                );
                BattlementGridAxis rowAxis = BattlementGridTrackSizing.Resolve(
                    rows,
                    autoRows,
                    placement.Rows,
                    rowGap,
                    Available(owner, horizontal: false),
                    children
                        .Select(
                            (child, index) =>
                                new BattlementGridContribution(
                                    placement.Items[index].Row,
                                    placement.Items[index].RowSpan,
                                    PreferredOuterHeight(
                                        child,
                                        AreaSize(
                                            columnAxis,
                                            placement.Items[index].Column,
                                            placement.Items[index].ColumnSpan,
                                            columnGap
                                        )
                                    )
                                )
                        )
                        .ToArray()
                );
                Apply(children, items, placement.Items, columnAxis, rowAxis);
            }
            catch (Exception exception)
                when (exception is InvalidOperationException or OverflowException)
            {
                EmitDiagnostic();
            }
        }

        private void Apply(
            IReadOnlyList<VisualElement> children,
            IReadOnlyList<GridItem> items,
            IReadOnlyList<BattlementGridPlacement> placements,
            BattlementGridAxis columnAxis,
            BattlementGridAxis rowAxis
        )
        {
            GridSignature next = new(
                children,
                items,
                placements,
                columnAxis.Sizes,
                rowAxis.Sizes,
                columnAxis.Total,
                rowAxis.Total,
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
            measurement.style.width = columnAxis.Total;
            measurement.style.height = rowAxis.Total;
            for (int index = 0; index < children.Count; index++)
            {
                BattlementGridPlacement placement = placements[index];
                Place(
                    adapter.SlotFor(children[index]),
                    children[index],
                    items[index],
                    columnAxis.Positions[placement.Column],
                    rowAxis.Positions[placement.Row],
                    AreaSize(columnAxis, placement.Column, placement.ColumnSpan, columnGap),
                    AreaSize(rowAxis, placement.Row, placement.RowSpan, rowGap)
                );
            }
            signature = next;
            unstablePasses++;
            PassCount++;
        }

        private void Place(
            BattlementLayoutSlot slot,
            VisualElement child,
            GridItem item,
            float left,
            float top,
            float width,
            float height
        )
        {
            float marginLeft = Margin(child.style.marginLeft);
            float marginRight = Margin(child.style.marginRight);
            float marginTop = Margin(child.style.marginTop);
            float marginBottom = Margin(child.style.marginBottom);
            left += marginLeft;
            top += marginTop;
            width = Math.Max(0, width - marginLeft - marginRight);
            height = Math.Max(0, height - marginTop - marginBottom);
            float preferredWidth =
                AuthoredExtent(child.style.width, width) ?? PreferredWidth(child);
            float preferredHeight =
                AuthoredExtent(child.style.height, height) ?? PreferredHeight(child, width);
            UiAlign horizontal = item.JustifySelf == UiAlign.Auto ? justifyItems : item.JustifySelf;
            UiAlign vertical = item.AlignSelf == UiAlign.Auto ? alignItems : item.AlignSelf;
            (left, width) = Align(
                left,
                width,
                preferredWidth,
                horizontal,
                AuthoredExtent(child.style.width, width).HasValue
            );
            (top, height) = Align(
                top,
                height,
                preferredHeight,
                vertical,
                AuthoredExtent(child.style.height, height).HasValue
            );
            slot.style.position = Position.Absolute;
            slot.style.left = left - marginLeft;
            slot.style.top = top - marginTop;
            slot.style.width = width + marginLeft + marginRight;
            slot.style.height = height + marginTop + marginBottom;
        }

        private static (float Position, float Size) Align(
            float position,
            float available,
            float preferred,
            UiAlign align,
            bool authoredSize
        )
        {
            if (align == UiAlign.Stretch && !authoredSize)
                return (position, available);
            return align switch
            {
                UiAlign.Center => (position + (available - preferred) / 2, preferred),
                UiAlign.FlexEnd => (position + available - preferred, preferred),
                _ => (position, preferred),
            };
        }

        private static float PreferredWidth(VisualElement child) =>
            AuthoredPixels(child.style.width)
            ?? FinitePositive(child.layout.width)
            ?? FinitePositive(child.resolvedStyle.width)
            ?? 0;

        private static float PreferredOuterWidth(VisualElement child) =>
            PreferredWidth(child)
            + Margin(child.style.marginLeft)
            + Margin(child.style.marginRight);

        private static float PreferredOuterHeight(VisualElement child, float width) =>
            PreferredHeight(
                child,
                Math.Max(
                    0,
                    width - Margin(child.style.marginLeft) - Margin(child.style.marginRight)
                )
            )
            + Margin(child.style.marginTop)
            + Margin(child.style.marginBottom);

        private static float PreferredHeight(VisualElement child, float width)
        {
            if (AuthoredPixels(child.style.height) is float authored)
                return authored;
            if (
                child is TextElement text
                && !string.IsNullOrEmpty(text.text)
                && child.panel is not null
            )
            {
                Vector2 measured = text.MeasureTextSize(
                    text.text,
                    Math.Max(0, width),
                    VisualElement.MeasureMode.Exactly,
                    0,
                    VisualElement.MeasureMode.Undefined
                );
                if (FinitePositive(measured.y) is float textHeight)
                    return textHeight;
            }
            if (child.style.height.value.unit == LengthUnit.Percent)
                return IntrinsicContentHeight(child);
            return FinitePositive(child.layout.height)
                ?? FinitePositive(child.resolvedStyle.height)
                ?? 0;
        }

        private static float IntrinsicContentHeight(VisualElement child)
        {
            float[] heights = child
                .Children()
                .Where(item => item.resolvedStyle.position != Position.Absolute)
                .Select(item => PreferredOuterHeight(item, child.contentRect.width))
                .ToArray();
            bool row =
                child.resolvedStyle.flexDirection is FlexDirection.Row or FlexDirection.RowReverse;
            float content = row ? heights.DefaultIfEmpty().Max() : heights.Sum();
            return content + Insets(child, horizontal: false);
        }

        private static float? Available(VisualElement value, bool horizontal)
        {
            StyleLength dimension = horizontal ? value.style.width : value.style.height;
            float insets = Insets(value, horizontal);
            if (AuthoredPixels(dimension) is float authored)
                return Math.Max(0, authored - insets);
            if (dimension.value.unit != LengthUnit.Percent && IntrinsicAxis(value, horizontal))
            {
                float? minimum = AuthoredPixels(
                    horizontal ? value.style.minWidth : value.style.minHeight
                );
                return minimum.HasValue ? Math.Max(0, minimum.Value - insets) : null;
            }
            float resolved = horizontal ? value.contentRect.width : value.contentRect.height;
            return value.panel is null ? null : FinitePositive(resolved);
        }

        private static bool IntrinsicAxis(VisualElement value, bool horizontal)
        {
            if (value.parent is null)
                return true;
            IResolvedStyle parent = value.parent.resolvedStyle;
            bool row = parent.flexDirection is FlexDirection.Row or FlexDirection.RowReverse;
            if (horizontal == row)
                return value.resolvedStyle.flexGrow == 0;
            UnityEngine.UIElements.Align alignment = value.resolvedStyle.alignSelf;
            if (alignment == UnityEngine.UIElements.Align.Auto)
                alignment = parent.alignItems;
            return alignment != UnityEngine.UIElements.Align.Stretch;
        }

        private static float Insets(VisualElement value, bool horizontal)
        {
            IResolvedStyle style = value.resolvedStyle;
            float extent = horizontal
                ? style.paddingLeft
                    + style.paddingRight
                    + style.borderLeftWidth
                    + style.borderRightWidth
                : style.paddingTop
                    + style.paddingBottom
                    + style.borderTopWidth
                    + style.borderBottomWidth;
            return float.IsFinite(extent) ? extent : 0;
        }

        private static float? AuthoredExtent(StyleLength value, float available)
        {
            if (value.keyword != StyleKeyword.Undefined || !float.IsFinite(value.value.value))
                return null;
            return value.value.unit == LengthUnit.Percent
                ? Math.Max(0, available * value.value.value / 100)
                : value.value.value;
        }

        private static float? AuthoredPixels(StyleLength value) =>
            value.keyword == StyleKeyword.Undefined
            && value.value.unit == LengthUnit.Pixel
            && float.IsFinite(value.value.value)
                ? value.value.value
                : null;

        private static float? FinitePositive(float value) =>
            float.IsFinite(value) && value > 0 ? value : null;

        private static float Margin(StyleLength value) => AuthoredPixels(value) ?? 0;

        private static float AreaSize(BattlementGridAxis axis, int start, int span, float gap) =>
            axis.Sizes.Skip(start).Take(span).Sum() + gap * Math.Max(0, span - 1);

        private void EmitDiagnostic()
        {
            if (diagnosticIssued)
                return;
            diagnosticIssued = true;
            DiagnosticCount++;
        }

        private static IReadOnlyList<GridTrack> Resolve(
            Prop<IReadOnlyList<GridTrack>> value,
            IReadOnlyList<GridTrack> current
        ) =>
            value.IsSet ? value.Value.ToArray()
            : value.IsReset ? Array.Empty<GridTrack>()
            : current;

        private static T Resolve<T>(Prop<T> value, T current, T reset) =>
            value.IsSet ? value.Value
            : value.IsReset ? reset
            : current;

        private sealed class GridSignature : IEquatable<GridSignature>
        {
            private readonly UiAlign alignItems;
            private readonly VisualElement[] children;
            private readonly float[] columns;
            private readonly float height;
            private readonly GridItem[] items;
            private readonly UiAlign justifyItems;
            private readonly BattlementGridPlacement[] placements;
            private readonly float[] rows;
            private readonly float width;

            public GridSignature(
                IReadOnlyList<VisualElement> children,
                IReadOnlyList<GridItem> items,
                IReadOnlyList<BattlementGridPlacement> placements,
                IReadOnlyList<float> columns,
                IReadOnlyList<float> rows,
                float width,
                float height,
                UiAlign alignItems,
                UiAlign justifyItems
            )
            {
                this.children = children.ToArray();
                this.items = items.ToArray();
                this.placements = placements.ToArray();
                this.columns = columns.ToArray();
                this.rows = rows.ToArray();
                this.width = width;
                this.height = height;
                this.alignItems = alignItems;
                this.justifyItems = justifyItems;
            }

            public bool Equals(GridSignature? other) =>
                other is not null
                && width.Equals(other.width)
                && height.Equals(other.height)
                && alignItems == other.alignItems
                && justifyItems == other.justifyItems
                && children.SequenceEqual(other.children)
                && items.SequenceEqual(other.items)
                && placements.SequenceEqual(other.placements)
                && columns.SequenceEqual(other.columns)
                && rows.SequenceEqual(other.rows);
        }
    }
}
