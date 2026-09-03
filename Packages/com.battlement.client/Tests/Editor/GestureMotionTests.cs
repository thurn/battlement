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
    public sealed class GestureMotionTests
    {
        [Test]
        public void ComposedButtonContentRecognizesTapsWithoutDragPropagation()
        {
            ObjectId host = Id("74400000-0000-4000-8000-000000000001");
            using var panel = new PanelFixture();
            var target = new Button();
            var content = new VisualElement();
            var image = new Image();
            content.Add(image);
            target.Add(content);
            panel.Root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                Descriptor(
                    Id("74400000-0000-4000-8000-000000000002"),
                    host,
                    1,
                    Id("74400000-0000-4000-8000-000000000003"),
                    Id("74400000-0000-4000-8000-000000000004"),
                    Id("74400000-0000-4000-8000-000000000005")
                )
            );

            PointerDown(image, Vector2.zero);
            IReadOnlyList<MotionGestureEventKind> started = Kinds(world);
            Assert.That(started, Has.Some.EqualTo(MotionGestureEventKind.TapStart));
            Assert.That(started, Has.None.EqualTo(MotionGestureEventKind.PanSessionStart));
            PointerUp(image, Vector2.zero);
            Assert.That(Kinds(world), Does.Contain(MotionGestureEventKind.Tap));

            PointerDown(image, Vector2.zero);
            _ = Kinds(world);
            PointerMove(image, new Vector2(20, 0));
            IReadOnlyList<MotionGestureEventKind> moved = Kinds(world);
            Assert.That(moved, Has.Some.EqualTo(MotionGestureEventKind.TapCancel));
            Assert.That(moved, Has.None.EqualTo(MotionGestureEventKind.PanStart));
            Assert.That(moved, Has.None.EqualTo(MotionGestureEventKind.DragStart));
            PointerUp(image, new Vector2(20, 0));
            _ = Kinds(world);

            var nestedControl = new Button();
            content.Add(nestedControl);
            PointerDown(nestedControl, Vector2.zero);
            Assert.That(Kinds(world), Has.None.EqualTo(MotionGestureEventKind.TapStart));
            PointerUp(nestedControl, Vector2.zero);
        }

        [Test]
        public void PointerThresholdDirectionConstraintAndMomentumStayNative()
        {
            double time = 0;
            ObjectId host = Id("74100000-0000-4000-8000-000000000001");
            ObjectId descriptorId = Id("74100000-0000-4000-8000-000000000002");
            ObjectId xValue = Id("74100000-0000-4000-8000-000000000003");
            ObjectId yValue = Id("74100000-0000-4000-8000-000000000004");
            ObjectId ySubscription = Id("74100000-0000-4000-8000-000000000005");
            using var panel = new PanelFixture();
            var target = new VisualElement();
            var pointerSurface = new VisualElement();
            target.Add(pointerSurface);
            panel.Root.Add(target);
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                gestureTime: () => TimeSpan.FromSeconds(time)
            );
            world.Install(
                target,
                host,
                Descriptor(descriptorId, host, 1, xValue, yValue, ySubscription, propagation: true)
            );

            PointerDown(pointerSurface, new Vector2(0, 0));
            time = 0.01;
            PointerMove(pointerSurface, new Vector2(2.9f, 0));
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.Zero);
            Assert.That(Kinds(world), Has.None.EqualTo(MotionGestureEventKind.DragStart));

            time = 0.02;
            PointerMove(pointerSurface, new Vector2(4, 0));
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(4).Within(0.001));
            Assert.That(Kinds(world), Does.Contain(MotionGestureEventKind.DragStart));

            time = 0.03;
            PointerMove(pointerSurface, new Vector2(20, 12));
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.Zero);
            Assert.That(ReadPixels(target, MotionProperty.Y), Is.EqualTo(9).Within(0.001));
            world.PreLayout();
            MotionEventBatch samples = world.DrainEventBatch()!;
            Assert.That(
                samples.ValueSamples!.Single(value => value.SubscriptionId == ySubscription).Value,
                Is.EqualTo(new MotionValue.Scalar(9))
            );
            Assert.That(
                samples.GestureEvents!.Select(value => value.Kind),
                Does.Contain(MotionGestureEventKind.DragDirectionLock)
            );

            PointerUp(pointerSurface, new Vector2(20, 12));
            MotionEventBatch ended = world.DrainEventBatch()!;
            MotionGestureEvent dragEnd = ended.GestureEvents!.Single(value =>
                value.Kind == MotionGestureEventKind.DragEnd
            );
            Assert.That(dragEnd.MomentumGeneration, Is.GreaterThan(0));

            time = 0.04;
            PointerDown(pointerSurface, new Vector2(20, 12));
            Assert.That(Kinds(world), Does.Contain(MotionGestureEventKind.DragCancel));
        }

        [Test]
        public void ExternalControlsScrollValuesAndReconnectCancellationAreCoherent()
        {
            double time = 0;
            ObjectId host = Id("74200000-0000-4000-8000-000000000001");
            ObjectId descriptorId = Id("74200000-0000-4000-8000-000000000002");
            ObjectId xValue = Id("74200000-0000-4000-8000-000000000003");
            ObjectId yValue = Id("74200000-0000-4000-8000-000000000004");
            ObjectId subscription = Id("74200000-0000-4000-8000-000000000005");
            ObjectId controls = Id("74200000-0000-4000-8000-000000000006");
            using var panel = new PanelFixture();
            var target = new VisualElement();
            panel.Root.Add(target);
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                gestureTime: () => TimeSpan.FromSeconds(time)
            );
            world.Install(
                target,
                host,
                Descriptor(
                    descriptorId,
                    host,
                    1,
                    xValue,
                    yValue,
                    subscription,
                    controls: controls,
                    constrained: false
                )
            );
            world.Apply(
                new MotionDragControlOperation(
                    controls,
                    7,
                    MotionPointerDevice.Mouse,
                    new MotionGestureVector(30, 20),
                    true
                )
            );
            Assert.That(target.style.translate.value.x.value, Is.EqualTo(30).Within(0.001));
            Assert.That(Kinds(world), Does.Contain(MotionGestureEventKind.DragStart));

            world.Install(
                target,
                host,
                Descriptor(
                    descriptorId,
                    host,
                    2,
                    xValue,
                    yValue,
                    subscription,
                    controls: controls,
                    constrained: false
                )
            );
            Assert.That(Kinds(world), Does.Contain(MotionGestureEventKind.DragCancel));

            ObjectId scrollHost = Id("74200000-0000-4000-8000-000000000007");
            ObjectId scrollDescriptor = Id("74200000-0000-4000-8000-000000000008");
            ObjectId scrollValue = Id("74200000-0000-4000-8000-000000000009");
            ObjectId scrollSubscription = Id("74200000-0000-4000-8000-00000000000a");
            var scroll = new ScrollView();
            panel.Root.Add(scroll);
            world.Install(
                scroll,
                scrollHost,
                ScrollDescriptor(scrollDescriptor, scrollHost, scrollValue, scrollSubscription)
            );
            scroll.scrollOffset = new Vector2(0, 42);
            world.PostLayout();
            world.PreLayout();
            MotionEventBatch batch = world.DrainEventBatch()!;
            Assert.That(
                batch
                    .GestureEvents!.Single(value => value.Kind == MotionGestureEventKind.Scroll)
                    .Offset.Y,
                Is.EqualTo(42)
            );
            Assert.That(
                batch
                    .ValueSamples!.Single(value => value.SubscriptionId == scrollSubscription)
                    .Value,
                Is.EqualTo(new MotionValue.Scalar(42))
            );
        }

        [Test]
        public void MousePenTouchKeyboardAndGamepadDeviceIdentityIsExplicit()
        {
            Assert.That(
                BattlementGestureState.Device(UnityEngine.UIElements.PointerType.mouse),
                Is.EqualTo(MotionPointerDevice.Mouse)
            );
            Assert.That(
                BattlementGestureState.Device(UnityEngine.UIElements.PointerType.pen),
                Is.EqualTo(MotionPointerDevice.Pen)
            );
            Assert.That(
                BattlementGestureState.Device(UnityEngine.UIElements.PointerType.touch),
                Is.EqualTo(MotionPointerDevice.Touch)
            );

            ObjectId host = Id("74300000-0000-4000-8000-000000000001");
            using var panel = new PanelFixture();
            var target = new VisualElement { focusable = true };
            panel.Root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                TapDescriptor(Id("74300000-0000-4000-8000-000000000002"), host)
            );
            using (
                KeyDownEvent key = KeyDownEvent.GetPooled('\n', KeyCode.Return, EventModifiers.None)
            )
            {
                key.target = target;
                target.SendEvent(key);
            }
            Assert.That(
                world
                    .DrainEventBatch()!
                    .GestureEvents!.Single(value => value.Kind == MotionGestureEventKind.Tap)
                    .Device,
                Is.EqualTo(MotionPointerDevice.Keyboard)
            );
            using (NavigationSubmitEvent submit = NavigationSubmitEvent.GetPooled())
            {
                submit.target = target;
                target.SendEvent(submit);
            }
            Assert.That(
                world
                    .DrainEventBatch()!
                    .GestureEvents!.Single(value => value.Kind == MotionGestureEventKind.Tap)
                    .Device,
                Is.EqualTo(MotionPointerDevice.Gamepad)
            );
        }

        private static MotionDescriptor Descriptor(
            ObjectId descriptorId,
            ObjectId host,
            uint generation,
            ObjectId xValue,
            ObjectId yValue,
            ObjectId ySubscription,
            bool propagation = false,
            ObjectId? controls = null,
            bool constrained = true
        ) =>
            new(
                descriptorId,
                host,
                generation,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Unscaled(),
                ReducedMotionPolicy.Never,
                Values: new[] { Mutable(xValue), Mutable(yValue) },
                ValueSubscriptions: new[]
                {
                    new MotionValueSubscription(ySubscription, yValue, MotionValueEventKind.Change),
                },
                Gestures: new MotionGestureDescriptor(
                    3,
                    10,
                    3,
                    8,
                    true,
                    new MotionDragDescriptor(
                        MotionGestureAxis.Both,
                        constrained
                            ? new MotionDragConstraint.Bounds(new MotionDragBounds(-5, 5, -8, 8))
                            : null,
                        new MotionDragElastic(0.25f, 0.25f, 0.25f, 0.25f),
                        true,
                        true,
                        true,
                        null,
                        controls,
                        propagation,
                        new MotionDragTransition(0.02f, 8, 500, 40),
                        xValue,
                        yValue
                    ),
                    false,
                    false,
                    null,
                    null,
                    null,
                    Subscriptions()
                )
            );

        private static MotionDescriptor ScrollDescriptor(
            ObjectId descriptorId,
            ObjectId host,
            ObjectId value,
            ObjectId subscription
        ) =>
            new(
                descriptorId,
                host,
                1,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Unscaled(),
                ReducedMotionPolicy.Never,
                Values: new[] { Mutable(value) },
                ValueSubscriptions: new[]
                {
                    new MotionValueSubscription(subscription, value, MotionValueEventKind.Change),
                },
                Gestures: new MotionGestureDescriptor(
                    3,
                    10,
                    3,
                    8,
                    false,
                    null,
                    false,
                    true,
                    null,
                    value,
                    null,
                    Subscriptions()
                )
            );

        private static MotionDescriptor TapDescriptor(ObjectId descriptorId, ObjectId host) =>
            new(
                descriptorId,
                host,
                1,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Unscaled(),
                ReducedMotionPolicy.Never,
                Gestures: new MotionGestureDescriptor(
                    3,
                    10,
                    3,
                    8,
                    false,
                    null,
                    false,
                    false,
                    null,
                    null,
                    null,
                    Subscriptions()
                )
            );

        private static MotionGestureSubscriptions Subscriptions() =>
            new(true, true, true, true, true, true, true, true, true, true, true);

        private static MotionValueDescriptor Mutable(ObjectId value) =>
            new(value, new MotionValue.Scalar(0), new MotionValueSource.Mutable());

        private static IReadOnlyList<MotionGestureEventKind> Kinds(BattlementMotionWorld world) =>
            world.DrainEventBatch()?.GestureEvents?.Select(value => value.Kind).ToArray()
            ?? Array.Empty<MotionGestureEventKind>();

        private static float ReadPixels(VisualElement target, MotionProperty property) =>
            BattlementMotionPropertyWriter.Read(target, property) is MotionValue.Length value
                ? (float)value.Value.Pixels
                : 0;

        private static void PointerDown(VisualElement target, Vector2 position)
        {
            using PointerDownEvent value = PointerDownEvent.GetPooled(
                new Event
                {
                    type = EventType.MouseDown,
                    button = 0,
                    mousePosition = position,
                }
            );
            value.target = target;
            target.SendEvent(value);
        }

        private static void PointerMove(VisualElement target, Vector2 position)
        {
            using PointerMoveEvent value = PointerMoveEvent.GetPooled(
                new Event { type = EventType.MouseMove, mousePosition = position }
            );
            value.target = target;
            target.SendEvent(value);
        }

        private static void PointerUp(VisualElement target, Vector2 position)
        {
            using PointerUpEvent value = PointerUpEvent.GetPooled(
                new Event
                {
                    type = EventType.MouseUp,
                    button = 0,
                    mousePosition = position,
                }
            );
            value.target = target;
            target.SendEvent(value);
        }

        private sealed class PanelFixture : IDisposable
        {
            private readonly GameObject gameObject;
            private readonly PanelSettings settings;

            public PanelFixture()
            {
                gameObject = new GameObject("Gesture Motion Test Panel");
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

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
