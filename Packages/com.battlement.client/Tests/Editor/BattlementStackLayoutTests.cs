#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class BattlementStackLayoutTests
    {
        [TestCase(true)]
        [TestCase(false)]
        public void DetachedTextDoesNotAttemptPanelDependentMeasurement(bool grid)
        {
            var warnings = new List<string>();
            void Record(string message, string trace, LogType type)
            {
                if (type == LogType.Warning)
                    warnings.Add(message);
            }
            Application.logMessageReceived += Record;
            try
            {
                var container = new BattlementLayoutContainer(
                    grid ? BattlementLayoutContainerKind.Grid : BattlementLayoutContainerKind.Stack
                );
                container.Adapter.Insert(new Label("Readable settings"), 0);
                Assert.That(warnings, Is.Empty);
            }
            finally
            {
                Application.logMessageReceived -= Record;
            }
        }

        [Test]
        public void PresentationSortsByOrderThenCurrentLogicalIndex()
        {
            var container = Stack(100, 60);
            var first = Sized(20, 10);
            var second = Sized(20, 10);
            var third = Sized(20, 10);
            BattlementStackItems.Apply(first, Item(order: 2));
            BattlementStackItems.Apply(second, Item(order: -1));
            BattlementStackItems.Apply(third, Item(order: 2));
            container.Adapter.Insert(first, 0);
            container.Adapter.Insert(second, 1);
            container.Adapter.Insert(third, 2);

            Assert.That(Presented(container), Is.EqualTo(new[] { second, first, third }));

            container.Adapter.Reindex(third, 0);

            Assert.That(
                container.Adapter.LogicalChildren,
                Is.EqualTo(new[] { third, first, second })
            );
            Assert.That(Presented(container), Is.EqualTo(new[] { second, third, first }));
        }

        [Test]
        public void PortalTiesFollowOrdinaryChildrenAndSourceOrdinals()
        {
            var container = Stack(100, 60);
            var ordinary = new VisualElement();
            var laterPortal = new VisualElement();
            var earlierPortal = new VisualElement();
            container.Adapter.Insert(ordinary, 0);
            container.Adapter.AttachPortal(laterPortal, new BattlementPortalSourceOrdinal(0, 2));
            container.Adapter.AttachPortal(earlierPortal, new BattlementPortalSourceOrdinal(0, 1));

            Assert.That(
                container.Adapter.LogicalChildren,
                Is.EqualTo(new[] { ordinary, earlierPortal, laterPortal })
            );
            Assert.That(
                Presented(container),
                Is.EqualTo(new[] { ordinary, earlierPortal, laterPortal })
            );
        }

        [Test]
        public void InsetsMarginsAndAlignmentProduceExactSlots()
        {
            var container = Stack(100, 80);
            var child = Sized(20, 10);
            child.style.marginLeft = 3;
            child.style.marginRight = 5;
            child.style.marginTop = 2;
            child.style.marginBottom = 4;
            BattlementStackItems.Apply(
                child,
                Item(
                    alignSelf: UiAlign.FlexEnd,
                    justifySelf: UiAlign.Center,
                    top: 5,
                    right: 7,
                    bottom: 9,
                    left: 11
                )
            );
            container.Adapter.Insert(child, 0);

            AssertRect(container.Adapter.SlotFor(child), 41, 57, 20, 10);
        }

        [Test]
        public void IntrinsicSizeIgnoresNoncontributingAndHiddenLayersStillContribute()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var contributing = Sized(40, 20);
            contributing.style.visibility = Visibility.Hidden;
            BattlementStackItems.Apply(contributing, Item(top: 4, right: 3, bottom: 5, left: 2));
            var ignored = Sized(100, 90);
            BattlementStackItems.Apply(ignored, Item(contributesToSize: false));
            var absent = Sized(200, 180);
            absent.style.display = DisplayStyle.None;
            container.Adapter.Insert(contributing, 0);
            container.Adapter.Insert(ignored, 1);
            container.Adapter.Insert(absent, 2);

            Assert.That(Pixels(container.Adapter.Measurement!.style.width), Is.EqualTo(45));
            Assert.That(Pixels(container.Adapter.Measurement.style.height), Is.EqualTo(29));
            Assert.That(
                container.Adapter.SlotFor(absent).style.display.value,
                Is.EqualTo(DisplayStyle.None)
            );
        }

        [Test]
        public void AllNoncontributingLayersHaveZeroIntrinsicSize()
        {
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            var child = Sized(80, 70);
            BattlementStackItems.Apply(child, Item(contributesToSize: false));
            container.Adapter.Insert(child, 0);

            Assert.That(Pixels(container.Adapter.Measurement!.style.width), Is.Zero);
            Assert.That(Pixels(container.Adapter.Measurement.style.height), Is.Zero);
            AssertRect(container.Adapter.SlotFor(child), 0, 0, 80, 70);
        }

        [Test]
        public void FixedWidthMeasuresAutomaticTextHeightAtTheFinalWidth()
        {
            using var panel = new PanelFixture();
            var container = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            container.style.width = 60;
            panel.Root.Add(container);
            var child = new Label("wrapped intrinsic Stack text");
            container.Adapter.Insert(child, 0);
            float expected = child
                .MeasureTextSize(
                    child.text,
                    60,
                    VisualElement.MeasureMode.Exactly,
                    0,
                    VisualElement.MeasureMode.Undefined
                )
                .y;

            Assert.That(Pixels(container.Adapter.Measurement!.style.height), Is.EqualTo(expected));
            Assert.That(Pixels(container.Adapter.SlotFor(child).style.width), Is.EqualTo(60));
        }

        [Test]
        public void ResetItemAndContainerDefaultsReuseTheExistingSlot()
        {
            var container = Stack(100, 50, UiAlign.Center, UiAlign.Center);
            var child = Sized(20, 10);
            BattlementStackItems.Apply(child, Item(order: 5, top: 7, left: 9));
            container.Adapter.Insert(child, 0);
            BattlementLayoutSlot retained = container.Adapter.SlotFor(child);

            BattlementStackItems.Apply(child, Prop<StackItem>.Reset());
            container.ApplyStack(
                new UiElement.Stack
                {
                    AlignItems = Prop<UiAlign>.Reset(),
                    JustifyItems = Prop<UiAlign>.Reset(),
                }
            );

            Assert.That(container.Adapter.SlotFor(child), Is.SameAs(retained));
            AssertRect(retained, 0, 0, 20, 10);
        }

        [Test]
        public void NestedStackCannotEscapeItsParentLayer()
        {
            var outer = Stack(100, 100);
            var lower = new VisualElement();
            var nested = Stack(100, 100);
            var upper = new VisualElement();
            BattlementStackItems.Apply(lower, Item(order: -1));
            BattlementStackItems.Apply(nested, Item(order: 0));
            BattlementStackItems.Apply(upper, Item(order: 1));
            outer.Adapter.Insert(lower, 0);
            outer.Adapter.Insert(nested, 1);
            outer.Adapter.Insert(upper, 2);
            var deeplyOrdered = new VisualElement();
            BattlementStackItems.Apply(deeplyOrdered, Item(order: 100));
            nested.Adapter.Insert(deeplyOrdered, 0);

            Assert.That(Presented(outer), Is.EqualTo(new[] { lower, nested, upper }));
            Assert.That(deeplyOrdered.parent, Is.SameAs(nested.Adapter.SlotFor(deeplyOrdered)));
            Assert.That(nested.parent, Is.SameAs(outer.Adapter.SlotFor(nested)));
        }

        private static BattlementLayoutContainer Stack(
            float width,
            float height,
            UiAlign align = UiAlign.Stretch,
            UiAlign justify = UiAlign.Stretch
        )
        {
            var value = new BattlementLayoutContainer(BattlementLayoutContainerKind.Stack);
            value.style.width = width;
            value.style.height = height;
            value.ApplyStack(new UiElement.Stack { AlignItems = align, JustifyItems = justify });
            return value;
        }

        private static VisualElement Sized(float width, float height)
        {
            var value = new VisualElement();
            value.style.width = width;
            value.style.height = height;
            return value;
        }

        private static StackItem Item(
            int order = 0,
            UiAlign alignSelf = UiAlign.Auto,
            UiAlign justifySelf = UiAlign.Auto,
            float? top = null,
            float? right = null,
            float? bottom = null,
            float? left = null,
            bool contributesToSize = true
        ) => new(order, alignSelf, justifySelf, top, right, bottom, left, contributesToSize);

        private static VisualElement[] Presented(BattlementLayoutContainer container) =>
            Enumerable
                .Range(1, container.Adapter.LogicalChildren.Count)
                .Select(index => ((BattlementLayoutSlot)container.hierarchy[index]).Host)
                .ToArray();

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

        private sealed class PanelFixture : IDisposable
        {
            private readonly GameObject gameObject;
            private readonly PanelSettings settings;

            public PanelFixture()
            {
                gameObject = new GameObject("Stack Layout Test Panel");
                var document = gameObject.AddComponent<UIDocument>();
                settings = ScriptableObject.CreateInstance<PanelSettings>();
                document.panelSettings = settings;
                Root = document.rootVisualElement;
            }

            public VisualElement Root { get; }

            public void Dispose()
            {
                UnityEngine.Object.DestroyImmediate(gameObject);
                UnityEngine.Object.DestroyImmediate(settings);
            }
        }
    }
}
