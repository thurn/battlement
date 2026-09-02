#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementLayoutPerformanceTests
    {
        [Test]
        public void ThousandGridChildrenCommitInOneLayoutPassAndSettleWithoutAllocation()
        {
            var grid = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            var columns = new GridTrack[25];
            Array.Fill(columns, new GridTrack.Fraction(1));
            grid.ApplyGrid(
                new UiElement.Grid { Columns = columns, AutoRows = new GridTrack.Px(14) }
            );
            int initialPasses = grid.GridLayout!.PassCount;

            grid.BeginUpdate();
            for (int index = 0; index < 1_000; index++)
            {
                var child = new VisualElement();
                child.style.width = 12;
                child.style.height = 12;
                grid.Adapter.Insert(child, grid.Adapter.Count);
            }

            Assert.That(grid.GridLayout.PassCount, Is.EqualTo(initialPasses));
            grid.EndUpdate();

            Assert.That(grid.Adapter.Count, Is.EqualTo(1_000));
            Assert.That(grid.GridLayout.PassCount, Is.EqualTo(initialPasses + 1));
            Assert.That(grid.GridLayout.LastMeasuredItemAxes, Is.EqualTo(2_000));
            Assert.That(grid.TakeLayoutDirty(), Is.True);
            Assert.That(grid.TakeLayoutDirty(), Is.False);
            Assert.That(StableDirtyPollingAllocation(grid, 10_000), Is.Zero);
        }

        [Test]
        public void MixedReleaseFixtureTracksStickyRowsStacksAndAnchoredOverlays()
        {
            BattlementStickyCoordinator sticky = StickyFixture();
            var stacks = new BattlementLayoutContainer[12];
            for (int index = 0; index < stacks.Length; index++)
            {
                stacks[index] = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
                stacks[index].Adapter.Insert(new VisualElement(), 0);
                stacks[index].Adapter.Insert(new VisualElement(), 1);
            }
            BattlementOverlayCoordinator overlays = OverlayFixture();

            Assert.That(sticky.EntryCount, Is.EqualTo(100));
            Assert.That(stacks, Has.Length.EqualTo(12));
            Assert.That(overlays.EntryCount, Is.EqualTo(10));
            Assert.That(StableOverlayAllocation(overlays, 100), Is.Zero);
        }

        private static BattlementStickyCoordinator StickyFixture()
        {
            var coordinator = new BattlementStickyCoordinator();
            var scroll = new ScrollView();
            var content = new VisualElement();
            scroll.Add(content);
            for (int index = 0; index < 100; index++)
            {
                var row = new VisualElement();
                content.Add(row);
                coordinator.Apply(row, new Sticky(index % 4, null, null, null, index), index);
            }
            return coordinator;
        }

        private static BattlementOverlayCoordinator OverlayFixture()
        {
            var anchors = new Dictionary<Guid, VisualElement>();
            var ordinals = new Dictionary<VisualElement, int>();
            var host = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var coordinator = new BattlementOverlayCoordinator(
                id => anchors.GetValueOrDefault(id.Value),
                wrapper => ordinals[wrapper],
                (_, _) => false,
                _ => Array.Empty<VisualElement>()
            );
            for (int index = 0; index < 10; index++)
            {
                Guid anchorId = Guid.NewGuid();
                anchors.Add(anchorId, new VisualElement());
                var wrapper = new VisualElement();
                wrapper.style.width = 90;
                wrapper.style.height = 28;
                host.Adapter.Insert(wrapper, host.Adapter.Count);
                ordinals.Add(wrapper, index);
                coordinator.Apply(
                    wrapper,
                    new OverlayPlacement.Popover(
                        new ObjectId(anchorId),
                        new PopoverPlacement(
                            PlacementSide.Bottom,
                            PlacementAlign.Start,
                            0,
                            0,
                            4,
                            true,
                            true
                        )
                    )
                );
            }
            return coordinator;
        }

        private static long StableDirtyPollingAllocation(
            BattlementLayoutContainer grid,
            int iterations
        )
        {
            long before = GC.GetAllocatedBytesForCurrentThread();
            for (int index = 0; index < iterations; index++)
                grid.TakeLayoutDirty();
            return GC.GetAllocatedBytesForCurrentThread() - before;
        }

        private static long StableOverlayAllocation(
            BattlementOverlayCoordinator coordinator,
            int iterations
        )
        {
            coordinator.RefreshAll();
            long before = GC.GetAllocatedBytesForCurrentThread();
            for (int index = 0; index < iterations; index++)
                coordinator.RefreshAll();
            return GC.GetAllocatedBytesForCurrentThread() - before;
        }
    }
}
