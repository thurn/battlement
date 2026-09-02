#nullable enable

using System;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI.Tests
{
    public sealed class BattlementOverlayCoordinatorTests
    {
        [TestCase(PlacementSide.Top, PlacementAlign.Start, 40, 30)]
        [TestCase(PlacementSide.Top, PlacementAlign.Center, 40, 30)]
        [TestCase(PlacementSide.Top, PlacementAlign.End, 40, 30)]
        [TestCase(PlacementSide.Right, PlacementAlign.Start, 60, 40)]
        [TestCase(PlacementSide.Bottom, PlacementAlign.Start, 40, 50)]
        [TestCase(PlacementSide.Left, PlacementAlign.End, 20, 40)]
        public void RequestedSidesAndAlignmentsUsePhysicalPanelDirections(
            PlacementSide side,
            PlacementAlign align,
            float expectedX,
            float expectedY
        )
        {
            BattlementPopoverResult result = Place(side, align);

            Assert.That(result.Rect.x, Is.EqualTo(expectedX));
            Assert.That(result.Rect.y, Is.EqualTo(expectedY));
            Assert.That(result.Side, Is.EqualTo(side));
        }

        [Test]
        public void FlipChoosesStrictlyBetterOppositeAndReprojectsMainOffset()
        {
            var placement = new PopoverPlacement(
                PlacementSide.Bottom,
                PlacementAlign.Start,
                4,
                3,
                0,
                true,
                false
            );

            BattlementPopoverResult result = BattlementOverlayCoordinator.ResolvePopover(
                new UnityRect(0, 0, 100, 100),
                new UnityRect(40, 90, 20, 10),
                new Vector2(20, 20),
                placement
            );

            Assert.That(result.Side, Is.EqualTo(PlacementSide.Top));
            Assert.That(result.Rect, Is.EqualTo(new UnityRect(43, 66, 20, 20)));
        }

        [Test]
        public void EqualFlipOverflowKeepsRequestedSide()
        {
            var placement = new PopoverPlacement(
                PlacementSide.Bottom,
                PlacementAlign.Center,
                0,
                0,
                0,
                true,
                false
            );

            BattlementPopoverResult result = BattlementOverlayCoordinator.ResolvePopover(
                new UnityRect(0, 0, 100, 100),
                new UnityRect(40, 45, 20, 10),
                new Vector2(20, 100),
                placement
            );

            Assert.That(result.Side, Is.EqualTo(PlacementSide.Bottom));
        }

        [Test]
        public void ShiftUsesLeadingEdgeForOversizedPopover()
        {
            var placement = new PopoverPlacement(
                PlacementSide.Bottom,
                PlacementAlign.End,
                0,
                0,
                8,
                false,
                true
            );

            BattlementPopoverResult result = BattlementOverlayCoordinator.ResolvePopover(
                new UnityRect(0, 0, 100, 100),
                new UnityRect(80, 20, 10, 10),
                new Vector2(120, 20),
                placement
            );

            Assert.That(result.Rect.x, Is.EqualTo(8));
            Assert.That(result.Rect.width, Is.EqualTo(120));
        }

        [Test]
        public void ExcessivePaddingCollapsesEachAxisToHostCenter()
        {
            var placement = new PopoverPlacement(
                PlacementSide.Bottom,
                PlacementAlign.Start,
                0,
                0,
                1000,
                false,
                true
            );

            BattlementPopoverResult result = BattlementOverlayCoordinator.ResolvePopover(
                new UnityRect(10, 20, 80, 60),
                new UnityRect(20, 30, 10, 10),
                new Vector2(5, 5),
                placement
            );

            Assert.That(float.IsFinite(result.Rect.x), Is.True);
            Assert.That(float.IsFinite(result.Rect.y), Is.True);
            Assert.That(result.Rect.x, Is.EqualTo(50));
        }

        [Test]
        public void OverlayDescriptorResetRemovesRetainedState()
        {
            var target = new VisualElement();
            var descriptor = new OverlayPlacement.Layer(OverlayLayer.Popover);

            BattlementOverlayItems.Apply(target, Prop<OverlayPlacement>.Set(descriptor));
            Assert.That(BattlementOverlayItems.Get(target), Is.SameAs(descriptor));

            BattlementOverlayItems.Apply(target, Prop<OverlayPlacement>.Reset());
            Assert.That(BattlementOverlayItems.HasAuthored(target), Is.False);
        }

        [Test]
        public void LayerPlacementValidatesAgainstStackHost()
        {
            var coordinator = new BattlementOverlayCoordinator(
                _ => null,
                _ => 0,
                (_, _) => false,
                _ => Array.Empty<VisualElement>()
            );

            Assert.DoesNotThrow(() =>
                coordinator.Validate(
                    new ObjectId(Guid.NewGuid()),
                    new OverlayPlacement.Layer(OverlayLayer.Popover),
                    new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack),
                    (_, _) => false
                )
            );
        }

        [Test]
        public void ModalValidationAcceptsAPendingInitialFocusDescendant()
        {
            ObjectId wrapper = new(Guid.NewGuid());
            ObjectId initialFocus = new(Guid.NewGuid());
            var coordinator = new BattlementOverlayCoordinator(
                _ => null,
                _ => 0,
                (_, _) => false,
                _ => Array.Empty<VisualElement>()
            );

            Assert.DoesNotThrow(() =>
                coordinator.Validate(
                    wrapper,
                    new OverlayPlacement.Modal(initialFocus, null),
                    new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack),
                    (candidate, ancestor) =>
                        candidate == initialFocus.Value && ancestor == wrapper.Value,
                    id => id == initialFocus
                )
            );
        }

        private static BattlementPopoverResult Place(PlacementSide side, PlacementAlign align) =>
            BattlementOverlayCoordinator.ResolvePopover(
                new UnityRect(0, 0, 100, 100),
                new UnityRect(40, 40, 20, 10),
                new Vector2(20, 10),
                new PopoverPlacement(side, align, 0, 0, 0, false, false)
            );
    }
}
