#nullable enable

using System;
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
