#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementGridLayoutTests
    {
        [Test]
        public void TrackSizingPreservesBasesRatiosGapsAndFiniteOverflow()
        {
            BattlementGridAxis bounded = BattlementGridTrackSizing.Resolve(
                new GridTrack[]
                {
                    new GridTrack.Px(100),
                    new GridTrack.Auto(),
                    new GridTrack.Fraction(2),
                },
                new GridTrack.Auto(),
                3,
                10,
                400,
                new[]
                {
                    new BattlementGridContribution(1, 1, 80),
                    new BattlementGridContribution(2, 1, 50),
                }
            );
            Assert.That(bounded.Sizes, Is.EqualTo(new[] { 100, 80, 200 }));
            Assert.That(bounded.Positions, Is.EqualTo(new[] { 0, 110, 200 }));
            Assert.That(bounded.Total, Is.EqualTo(400));

            BattlementGridAxis shortage = BattlementGridTrackSizing.Resolve(
                new GridTrack[] { new GridTrack.Fraction(1), new GridTrack.Fraction(1) },
                new GridTrack.Auto(),
                2,
                0,
                200,
                new[]
                {
                    new BattlementGridContribution(0, 1, 150),
                    new BattlementGridContribution(1, 1, 150),
                }
            );
            Assert.That(shortage.Sizes, Is.EqualTo(new[] { 150, 150 }));
            Assert.That(shortage.Overflow, Is.EqualTo(100));
        }

        [Test]
        public void SpanningDeficitRaisesAutomaticTracksBeforeFractions()
        {
            BattlementGridAxis result = BattlementGridTrackSizing.Resolve(
                new GridTrack[] { new GridTrack.Auto(), new GridTrack.Fraction(1) },
                new GridTrack.Auto(),
                2,
                0,
                200,
                new[]
                {
                    new BattlementGridContribution(0, 1, 100),
                    new BattlementGridContribution(1, 1, 50),
                    new BattlementGridContribution(0, 2, 300),
                }
            );

            Assert.That(result.Sizes, Is.EqualTo(new[] { 250, 50 }));
            Assert.That(result.Overflow, Is.EqualTo(100));
        }

        [Test]
        public void MixedPlacementMatchesTheGoldenTableInBothFlowDirections()
        {
            GridItem[] items =
            {
                Item(row: 1, column: 2),
                Item(columnSpan: 2),
                Item(row: 1),
                Item(column: 1),
                Item(),
            };

            BattlementGridPlacementResult rows = BattlementGridOccupancy.Place(
                items,
                0,
                3,
                GridAutoFlow.Row
            );
            AssertPlacement(rows.Items[0], 0, 1, 1, 1);
            AssertPlacement(rows.Items[1], 1, 0, 1, 2);
            AssertPlacement(rows.Items[2], 0, 0, 1, 1);
            AssertPlacement(rows.Items[3], 2, 0, 1, 1);
            AssertPlacement(rows.Items[4], 2, 1, 1, 1);

            GridItem[] transposed = items
                .Select(item => new GridItem(
                    item.Column,
                    item.Row,
                    item.ColumnSpan,
                    item.RowSpan,
                    item.JustifySelf,
                    item.AlignSelf
                ))
                .ToArray();
            BattlementGridPlacementResult columns = BattlementGridOccupancy.Place(
                transposed,
                3,
                0,
                GridAutoFlow.Column
            );
            AssertPlacement(columns.Items[0], 1, 0, 1, 1);
            AssertPlacement(columns.Items[1], 0, 1, 2, 1);
            AssertPlacement(columns.Items[2], 0, 0, 1, 1);
            AssertPlacement(columns.Items[3], 0, 2, 1, 1);
            AssertPlacement(columns.Items[4], 1, 2, 1, 1);
        }

        [Test]
        public void AuthoredMinorStartUsesItsOccupiedFarEdge()
        {
            BattlementGridPlacementResult result = BattlementGridOccupancy.Place(
                new[] { Item(column: 4, columnSpan: 2) },
                0,
                0,
                GridAutoFlow.Row
            );

            Assert.That(result.Columns, Is.EqualTo(5));
            AssertPlacement(result.Items[0], 0, 3, 1, 2);
        }

        [Test]
        public void SpansCrossOneGapAndItemOverridesRespectMargins()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Px(40), new GridTrack.Px(50) },
                    Rows = new GridTrack[] { new GridTrack.Px(30) },
                    ColumnGap = 10,
                }
            );
            VisualElement child = Sized(20, 10);
            child.style.marginLeft = 5;
            child.style.marginRight = 7;
            child.style.marginTop = 3;
            BattlementGridItems.Apply(
                child,
                Item(columnSpan: 2, alignSelf: UiAlign.FlexEnd, justifySelf: UiAlign.Center)
            );
            container.Adapter.Insert(child, 0);

            AssertRect(container.Adapter.SlotFor(child), 34, 17, 32, 13);
        }

        [Test]
        public void ResetPlacementAndFlowReuseTheExistingSlot()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Px(20), new GridTrack.Px(20) },
                    Rows = new GridTrack[] { new GridTrack.Px(20), new GridTrack.Px(20) },
                    AutoFlow = GridAutoFlow.Column,
                }
            );
            var first = new VisualElement();
            var second = new VisualElement();
            BattlementGridItems.Apply(second, Item(row: 2, column: 2));
            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);
            BattlementLayoutSlot retained = container.Adapter.SlotFor(second);

            BattlementGridItems.Apply(second, Prop<GridItem>.Reset());
            container.ApplyGrid(new UiElement.Grid { AutoFlow = Prop<GridAutoFlow>.Reset() });

            Assert.That(container.Adapter.SlotFor(second), Is.SameAs(retained));
            AssertRect(retained, 20, 0, 20, 20);
        }

        [Test]
        public void DefaultRowFlowCreatesImplicitRowsAndPlacesAlignedSlots()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            container.style.width = 160;
            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Px(50), new GridTrack.Fraction(1) },
                    Rows = new GridTrack[] { new GridTrack.Px(20) },
                    AutoRows = new GridTrack.Px(30),
                    ColumnGap = 10,
                    RowGap = 5,
                    AlignItems = UiAlign.FlexEnd,
                    JustifyItems = UiAlign.Center,
                }
            );
            VisualElement[] children = { Sized(20, 10), Sized(20, 10), Sized(20, 10) };
            for (int index = 0; index < children.Length; index++)
                container.Adapter.Insert(children[index], index);

            AssertRect(container.Adapter.SlotFor(children[0]), 15, 10, 20, 10);
            AssertRect(container.Adapter.SlotFor(children[1]), 100, 10, 20, 10);
            AssertRect(container.Adapter.SlotFor(children[2]), 15, 45, 20, 10);
            Assert.That(Pixels(container.Adapter.Measurement!.style.width), Is.EqualTo(160));
            Assert.That(Pixels(container.Adapter.Measurement.style.height), Is.EqualTo(55));
        }

        [Test]
        public void EmptyColumnsCreateOneImplicitColumnAndResetsKeepSlotsStable()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = Array.Empty<GridTrack>(),
                    AutoColumns = new GridTrack.Px(40),
                    AutoRows = new GridTrack.Px(12),
                    RowGap = 3,
                }
            );
            var first = new VisualElement();
            var second = new VisualElement();
            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);
            BattlementLayoutSlot slot = container.Adapter.SlotFor(first);
            int settledPass = container.GridLayout!.PassCount;

            container.GridLayout.Refresh();
            Assert.That(container.GridLayout.PassCount, Is.EqualTo(settledPass));
            Assert.That(Pixels(container.Adapter.SlotFor(second).style.top), Is.EqualTo(15));

            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = Prop<System.Collections.Generic.IReadOnlyList<GridTrack>>.Reset(),
                    RowGap = Prop<float>.Reset(),
                    AlignItems = Prop<UiAlign>.Reset(),
                }
            );
            Assert.That(container.Adapter.SlotFor(first), Is.SameAs(slot));
            Assert.That(Pixels(container.Adapter.SlotFor(second).style.top), Is.EqualTo(12));
        }

        [Test]
        public void NonfinitePassRetainsTheLastLayoutAndReportsOnce()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            container.ApplyGrid(
                new UiElement.Grid
                {
                    Columns = new GridTrack[] { new GridTrack.Px(40) },
                    Rows = new GridTrack[] { new GridTrack.Px(20) },
                }
            );
            container.Adapter.Insert(new VisualElement(), 0);
            float retainedWidth = Pixels(container.Adapter.Measurement!.style.width);

            container.ApplyGrid(new UiElement.Grid { ColumnGap = float.NaN });
            container.GridLayout!.Refresh();

            Assert.That(container.GridLayout.DiagnosticCount, Is.EqualTo(1));
            Assert.That(
                Pixels(container.Adapter.Measurement.style.width),
                Is.EqualTo(retainedWidth)
            );
        }

        private static VisualElement Sized(float width, float height)
        {
            var value = new VisualElement();
            value.style.width = width;
            value.style.height = height;
            return value;
        }

        private static GridItem Item(
            uint? row = null,
            uint? column = null,
            uint rowSpan = 1,
            uint columnSpan = 1,
            UiAlign alignSelf = UiAlign.Auto,
            UiAlign justifySelf = UiAlign.Auto
        ) => new(row, column, rowSpan, columnSpan, alignSelf, justifySelf);

        private static void AssertPlacement(
            BattlementGridPlacement value,
            int row,
            int column,
            int rowSpan,
            int columnSpan
        )
        {
            Assert.That(value.Row, Is.EqualTo(row));
            Assert.That(value.Column, Is.EqualTo(column));
            Assert.That(value.RowSpan, Is.EqualTo(rowSpan));
            Assert.That(value.ColumnSpan, Is.EqualTo(columnSpan));
        }

        private static void AssertRect(
            BattlementLayoutSlot slot,
            float left,
            float top,
            float width,
            float height
        )
        {
            Assert.That(Pixels(slot.style.left), Is.EqualTo(left));
            Assert.That(Pixels(slot.style.top), Is.EqualTo(top));
            Assert.That(Pixels(slot.style.width), Is.EqualTo(width));
            Assert.That(Pixels(slot.style.height), Is.EqualTo(height));
        }

        private static float Pixels(StyleLength value) => value.value.value;
    }
}
