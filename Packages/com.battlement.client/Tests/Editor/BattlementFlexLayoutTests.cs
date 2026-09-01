#nullable enable

using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementFlexLayoutTests
    {
        [Test]
        public void GapsUseHalfMarginsWithoutOverwritingChildMargins()
        {
            BattlementLayoutContainer container = Container(rowGap: 8, columnGap: 12);
            var first = new VisualElement();
            first.style.marginLeft = 5;
            var second = new VisualElement();

            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);

            Assert.That(Pixels(Slot(container, first).style.marginLeft), Is.EqualTo(6));
            Assert.That(Pixels(Slot(container, first).style.marginTop), Is.EqualTo(4));
            Assert.That(Pixels(Slot(container, second).style.marginRight), Is.EqualTo(6));
            Assert.That(Pixels(container.FlexLayout!.FlowBand.style.marginLeft), Is.EqualTo(-6));
            Assert.That(Pixels(container.FlexLayout.FlowBand.style.marginTop), Is.EqualTo(-4));
            Assert.That(Pixels(first.style.marginLeft), Is.EqualTo(5));
        }

        [Test]
        public void EmptyHiddenAndAbsoluteOnlyFlowsDoNotCompensateTheOuterBand()
        {
            BattlementLayoutContainer container = Container(rowGap: 8, columnGap: 12);
            Assert.That(Pixels(container.FlexLayout!.FlowBand.style.marginLeft), Is.Zero);

            var hidden = new VisualElement();
            hidden.style.display = DisplayStyle.None;
            container.Adapter.Insert(hidden, 0);
            Assert.That(Pixels(container.FlexLayout.FlowBand.style.marginLeft), Is.Zero);

            var absolute = new VisualElement();
            absolute.style.position = Position.Absolute;
            absolute.style.left = 9;
            container.Adapter.Insert(absolute, 1);

            Assert.That(Pixels(container.FlexLayout.FlowBand.style.marginLeft), Is.Zero);
            Assert.That(Slot(container, absolute).parent, Is.SameAs(container));
            Assert.That(Pixels(Slot(container, absolute).style.marginLeft), Is.Zero);
            Assert.That(Pixels(Slot(container, absolute).style.left), Is.EqualTo(9));
        }

        [Test]
        public void ReverseAndWrapAffectPresentationWithoutChangingLogicalOrder()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Flex);
            var first = new VisualElement();
            var second = new VisualElement();
            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);

            container.ApplyFlex(
                new UiElement.Flex
                {
                    Direction = UiFlexDirection.RowReverse,
                    Wrap = UiFlexWrap.WrapReverse,
                }
            );

            Assert.That(container.Adapter.LogicalChildren, Is.EqualTo(new[] { first, second }));
            Assert.That(
                container.FlexLayout!.FlowBand.style.flexDirection.value,
                Is.EqualTo(FlexDirection.RowReverse)
            );
            Assert.That(
                container.FlexLayout.FlowBand.style.flexWrap.value,
                Is.EqualTo(Wrap.WrapReverse)
            );
            Assert.That(
                container.FlexLayout.FlowBand.hierarchy[0],
                Is.SameAs(Slot(container, first))
            );
        }

        [Test]
        public void SparseUpdatesAndResetsKeepSlotsStableAndWorkSettles()
        {
            BattlementLayoutContainer container = Container(rowGap: 3, columnGap: 5);
            var child = new VisualElement();
            container.Adapter.Insert(child, 0);
            VisualElement slot = Slot(container, child);
            int pass = container.FlexLayout!.PassCount;

            container.FlexLayout.Refresh();
            Assert.That(container.FlexLayout.PassCount, Is.EqualTo(pass));

            container.ApplyFlex(
                new UiElement.Flex { RowGap = Prop<float>.Reset(), Direction = UiFlexDirection.Row }
            );

            Assert.That(Slot(container, child), Is.SameAs(slot));
            Assert.That(Pixels(slot.style.marginTop), Is.Zero);
            Assert.That(Pixels(slot.style.marginLeft), Is.EqualTo(2.5f));
            Assert.That(container.TakeLayoutDirty(), Is.True);
            Assert.That(container.TakeLayoutDirty(), Is.False);
        }

        private static BattlementLayoutContainer Container(float rowGap, float columnGap)
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Flex);
            container.ApplyFlex(
                new UiElement.Flex
                {
                    Direction = UiFlexDirection.Row,
                    AlignItems = UiAlign.Stretch,
                    JustifyContent = UiJustify.FlexStart,
                    RowGap = rowGap,
                    ColumnGap = columnGap,
                }
            );
            return container;
        }

        private static BattlementLayoutSlot Slot(
            BattlementLayoutContainer container,
            VisualElement child
        ) => container.Adapter.SlotFor(child);

        private static float Pixels(StyleLength value) => value.value.value;
    }
}
