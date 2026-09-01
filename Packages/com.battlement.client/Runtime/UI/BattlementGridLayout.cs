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

        public int PassCount { get; private set; }

        public void Apply(UiElement.Grid value)
        {
            columns = Resolve(value.Columns, columns);
            rows = Resolve(value.Rows, rows);
            autoColumns = Resolve(value.AutoColumns, autoColumns, new GridTrack.Auto());
            autoRows = Resolve(value.AutoRows, autoRows, new GridTrack.Auto());
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
            int columnCount = Math.Max(columns.Count, 1);
            int rowCount = Math.Max(rows.Count, DivideCeiling(children.Length, columnCount));
            try
            {
                BattlementGridAxis columnAxis = BattlementGridTrackSizing.Resolve(
                    columns,
                    autoColumns,
                    columnCount,
                    columnGap,
                    Available(owner, horizontal: true),
                    children
                        .Select(
                            (child, index) =>
                                new BattlementGridContribution(
                                    index % columnCount,
                                    1,
                                    PreferredWidth(child)
                                )
                        )
                        .ToArray()
                );
                BattlementGridAxis rowAxis = BattlementGridTrackSizing.Resolve(
                    rows,
                    autoRows,
                    rowCount,
                    rowGap,
                    Available(owner, horizontal: false),
                    children
                        .Select(
                            (child, index) =>
                                new BattlementGridContribution(
                                    index / columnCount,
                                    1,
                                    PreferredHeight(child, columnAxis.Sizes[index % columnCount])
                                )
                        )
                        .ToArray()
                );
                Apply(children, columnCount, columnAxis, rowAxis);
            }
            catch (InvalidOperationException)
            {
                EmitDiagnostic();
            }
        }

        private void Apply(
            IReadOnlyList<VisualElement> children,
            int columnCount,
            BattlementGridAxis columnAxis,
            BattlementGridAxis rowAxis
        )
        {
            GridSignature next = new(
                children,
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
                int column = index % columnCount;
                int row = index / columnCount;
                Place(
                    adapter.SlotFor(children[index]),
                    children[index],
                    columnAxis.Positions[column],
                    rowAxis.Positions[row],
                    columnAxis.Sizes[column],
                    rowAxis.Sizes[row]
                );
            }
            signature = next;
            unstablePasses++;
            PassCount++;
        }

        private void Place(
            BattlementLayoutSlot slot,
            VisualElement child,
            float left,
            float top,
            float width,
            float height
        )
        {
            float preferredWidth = PreferredWidth(child);
            float preferredHeight = PreferredHeight(child, width);
            (left, width) = Align(left, width, preferredWidth, justifyItems, HasWidth(child));
            (top, height) = Align(top, height, preferredHeight, alignItems, HasHeight(child));
            slot.style.position = Position.Absolute;
            slot.style.left = left;
            slot.style.top = top;
            slot.style.width = width;
            slot.style.height = height;
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

        private static float PreferredHeight(VisualElement child, float width)
        {
            if (AuthoredPixels(child.style.height) is float authored)
                return authored;
            if (child is TextElement text && !string.IsNullOrEmpty(text.text))
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
            return FinitePositive(child.layout.height)
                ?? FinitePositive(child.resolvedStyle.height)
                ?? 0;
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

        private static float? AuthoredPixels(StyleLength value) =>
            value.keyword == StyleKeyword.Undefined
            && value.value.unit == LengthUnit.Pixel
            && float.IsFinite(value.value.value)
                ? value.value.value
                : null;

        private static float? FinitePositive(float value) =>
            float.IsFinite(value) && value > 0 ? value : null;

        private void EmitDiagnostic()
        {
            if (diagnosticIssued)
                return;
            diagnosticIssued = true;
            DiagnosticCount++;
        }

        private static int DivideCeiling(int value, int divisor) =>
            value == 0 ? 0 : (value - 1) / divisor + 1;

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
            private readonly UiAlign justifyItems;
            private readonly float[] rows;
            private readonly float width;

            public GridSignature(
                IReadOnlyList<VisualElement> children,
                IReadOnlyList<float> columns,
                IReadOnlyList<float> rows,
                float width,
                float height,
                UiAlign alignItems,
                UiAlign justifyItems
            )
            {
                this.children = children.ToArray();
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
                && columns.SequenceEqual(other.columns)
                && rows.SequenceEqual(other.rows);
        }
    }
}
