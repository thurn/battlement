#nullable enable

using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementLayoutContainerTests
    {
        [Test]
        public void PresentationSortingDoesNotChangeLogicalIndexMeaning()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var first = new Button { text = "first" };
            var second = new Button { text = "second" };
            var third = new Button { text = "third" };
            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);
            container.Adapter.Insert(third, 2);
            Assert.That(container.Adapter.TryGetSlot(first, out VisualElement? firstSlot), Is.True);

            container.Adapter.Present(new[] { third, first, second });

            Assert.That(
                container.Adapter.LogicalChildren,
                Is.EqualTo(new[] { first, second, third })
            );
            Assert.That(container.hierarchy[1], Is.SameAs(Slot(container, third)));
            Assert.That(container.hierarchy[2], Is.SameAs(firstSlot));

            container.Adapter.Reindex(third, 0);

            Assert.That(
                container.Adapter.LogicalChildren,
                Is.EqualTo(new[] { third, first, second })
            );
            Assert.That(Slot(container, first), Is.SameAs(firstSlot));
        }

        [Test]
        public void ReparentPreservesTheActualControlAndItsState()
        {
            var first = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            var second = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var field = new TextField();
            field.SetValueWithoutNotify("draft");
            first.Adapter.Insert(field, 0);
            VisualElement firstSlot = Slot(first, field);

            first.Adapter.Detach(field);
            second.Adapter.Insert(field, 0);

            Assert.That(field.value, Is.EqualTo("draft"));
            Assert.That(field.parent, Is.SameAs(Slot(second, field)));
            Assert.That(field.parent, Is.Not.SameAs(firstSlot));
            Assert.That(first.Adapter.LogicalChildren, Is.Empty);
            Assert.That(second.Adapter.LogicalChildren, Is.EqualTo(new[] { field }));
        }

        [Test]
        public void PortalAttachmentsFollowOneGlobalSourceOrdinal()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            var ordinary = new VisualElement();
            var laterRoot = new VisualElement();
            var laterPortal = new VisualElement();
            var earlierPortal = new VisualElement();
            container.Adapter.Insert(ordinary, 0);
            container.Adapter.AttachPortal(laterRoot, new BattlementPortalSourceOrdinal(1, 0));
            container.Adapter.AttachPortal(laterPortal, new BattlementPortalSourceOrdinal(0, 2));
            container.Adapter.AttachPortal(earlierPortal, new BattlementPortalSourceOrdinal(0, 1));

            Assert.That(
                container.Adapter.LogicalChildren,
                Is.EqualTo(new[] { ordinary, earlierPortal, laterPortal, laterRoot })
            );
            Assert.That(
                Enumerable
                    .Range(1, 4)
                    .Select(index => ((BattlementLayoutSlot)container.hierarchy[index]).Host),
                Is.EqualTo(container.Adapter.LogicalChildren)
            );
        }

        [Test]
        public void ClearRemovesEveryPrivateNodeAndScheduledDirtyStateSettles()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var child = new VisualElement();
            container.Adapter.Insert(child, 0);
            Assert.That(container.TakeLayoutDirty(), Is.True);
            Assert.That(container.TakeLayoutDirty(), Is.False);

            container.Adapter.Clear();

            Assert.That(child.parent, Is.Null);
            Assert.That(container.hierarchy.childCount, Is.Zero);
            Assert.That(container.Adapter.LogicalChildren, Is.Empty);
            Assert.That(container.TakeLayoutDirty(), Is.True);
            Assert.That(container.TakeLayoutDirty(), Is.False);
        }

        [Test]
        public void ReconstructionProducesTheSamePublicLogicalTree()
        {
            VisualElement[] firstHosts = { new Button(), new TextField(), new VisualElement() };
            VisualElement[] secondHosts = { new Button(), new TextField(), new VisualElement() };
            var first = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            var second = new BattlementLayoutContainer(BattlementLayoutContainerKind.Grid);
            for (int index = 0; index < firstHosts.Length; index++)
            {
                first.Adapter.Insert(firstHosts[index], index);
                second.Adapter.Insert(secondHosts[index], index);
            }

            Assert.That(
                second.Adapter.LogicalChildren.Select(value => value.GetType()),
                Is.EqualTo(first.Adapter.LogicalChildren.Select(value => value.GetType()))
            );
            Assert.That(first.Adapter.Measurement, Is.Not.Null);
            Assert.That(second.Adapter.Measurement, Is.Not.Null);
        }

        private static VisualElement Slot(BattlementLayoutContainer container, VisualElement child)
        {
            Assert.That(container.Adapter.TryGetSlot(child, out VisualElement? slot), Is.True);
            return slot!;
        }
    }
}
