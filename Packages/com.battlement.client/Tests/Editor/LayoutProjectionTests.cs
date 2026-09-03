#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class LayoutProjectionTests
    {
        [TestCase(MotionLayoutMode.Position, -50, -25, 1, 1)]
        [TestCase(MotionLayoutMode.Size, 0, 0, 0.75f, 0.75f)]
        [TestCase(MotionLayoutMode.Both, -75, -37.5f, 0.75f, 0.75f)]
        public void FrozenMidpointPreservesTheRequestedVisibleGeometry(
            MotionLayoutMode mode,
            float expectedX,
            float expectedY,
            float expectedScaleX,
            float expectedScaleY
        )
        {
            (Vector2 translation, Vector2 scale) = BattlementLayoutProjection.Resolve(
                new UnityEngine.Rect(10, 20, 100, 50),
                new UnityEngine.Rect(110, 70, 200, 100),
                mode,
                0.5f
            );

            Assert.That(translation.x, Is.EqualTo(expectedX).Within(0.001));
            Assert.That(translation.y, Is.EqualTo(expectedY).Within(0.001));
            Assert.That(scale.x, Is.EqualTo(expectedScaleX).Within(0.001));
            Assert.That(scale.y, Is.EqualTo(expectedScaleY).Within(0.001));
        }

        [Test]
        public void TerminalProjectionIsIdentityForEveryMode()
        {
            foreach (MotionLayoutMode mode in System.Enum.GetValues(typeof(MotionLayoutMode)))
            {
                (Vector2 translation, Vector2 scale) = BattlementLayoutProjection.Resolve(
                    new UnityEngine.Rect(0, 0, 80, 40),
                    new UnityEngine.Rect(200, 100, 240, 120),
                    mode,
                    1
                );
                Assert.That(translation, Is.EqualTo(Vector2.zero));
                Assert.That(scale, Is.EqualTo(Vector2.one));
            }
        }

        [TestCase(MotionLayoutMode.Position, 10, 20, 200, 100)]
        [TestCase(MotionLayoutMode.Size, 160, 95, 100, 50)]
        [TestCase(MotionLayoutMode.Both, 10, 20, 100, 50)]
        public void FirstProjectedFramePreservesTheSelectedOriginAxes(
            MotionLayoutMode mode,
            float x,
            float y,
            float width,
            float height
        )
        {
            UnityEngine.Rect projected = BattlementLayoutProjection.ProjectedBounds(
                new UnityEngine.Rect(10, 20, 100, 50),
                new UnityEngine.Rect(110, 70, 200, 100),
                mode,
                0
            );

            Assert.That(projected, Is.EqualTo(new UnityEngine.Rect(x, y, width, height)));
        }

        [Test]
        public void InterruptionRetargetsContinuouslyFromTheVisibleMidpoint()
        {
            UnityEngine.Rect start = new(10, 20, 100, 50);
            UnityEngine.Rect destination = new(110, 70, 200, 100);
            UnityEngine.Rect visible = BattlementLayoutProjection.ProjectedBounds(
                start,
                destination,
                MotionLayoutMode.Both,
                0.5f
            );
            UnityEngine.Rect interrupted = BattlementLayoutProjection.ProjectedBounds(
                visible,
                start,
                MotionLayoutMode.Both,
                0
            );

            Assert.That(interrupted, Is.EqualTo(visible));
            Assert.That(visible, Is.Not.EqualTo(start));
            Assert.That(visible, Is.Not.EqualTo(destination));
        }

        [Test]
        public void ScaleCorrectionPreservesAChildTransformThatChangesMidProjection()
        {
            Vector2 unchanged = BattlementLayoutProjection.ComposeScaleCorrection(
                new Vector2(1, 0.75f),
                new Vector2(0.5f, 0.25f),
                new Vector2(1, 0.75f),
                new Vector2(4, 2)
            );
            Vector2 animated = BattlementLayoutProjection.ComposeScaleCorrection(
                new Vector2(2.4f, 3.2f),
                new Vector2(0.5f, 0.25f),
                new Vector2(1, 0.75f),
                new Vector2(4, 2)
            );

            Assert.That(unchanged, Is.EqualTo(new Vector2(0.5f, 1.5f)));
            Assert.That(animated, Is.EqualTo(new Vector2(0.6f, 1.6f)));
        }

        [Test]
        public void SharedLayoutAcceptsSamePanelPortalsAndDepartedPresenceHandoffs()
        {
            using var panel = new PanelFixture();
            var source = new VisualElement();
            var portal = new VisualElement();
            var destination = new VisualElement();
            panel.Root.Add(source);
            panel.Root.Add(portal);
            portal.Add(destination);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            ObjectId sourceHost = Id("75200000-0000-4000-8000-000000000001");
            ObjectId destinationHost = Id("75200000-0000-4000-8000-000000000002");

            world.Install(source, sourceHost, Descriptor(sourceHost, 1));
            Assert.DoesNotThrow(() =>
                world.Install(destination, destinationHost, Descriptor(destinationHost, 2))
            );
            world.RemoveHost(destinationHost);
            world.RemoveHost(sourceHost);
            Assert.DoesNotThrow(() =>
                world.Install(destination, destinationHost, Descriptor(destinationHost, 3))
            );
        }

        [Test]
        public void SharedLayoutIgnoresProjectionCandidatesWithoutSharedIdentity()
        {
            using var panel = new PanelFixture();
            var ordinary = new VisualElement();
            var shared = new VisualElement();
            panel.Root.Add(ordinary);
            panel.Root.Add(shared);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            ObjectId ordinaryHost = Id("75200000-0000-4000-8000-000000000011");
            ObjectId sharedHost = Id("75200000-0000-4000-8000-000000000012");
            MotionDescriptor ordinaryDescriptor = Descriptor(ordinaryHost, 1) with
            {
                Layout = Descriptor(ordinaryHost, 1).Layout! with { LayoutId = null },
            };

            world.Install(ordinary, ordinaryHost, ordinaryDescriptor);

            Assert.DoesNotThrow(() => world.Install(shared, sharedHost, Descriptor(sharedHost, 1)));
        }

        [Test]
        public void SharedLayoutRejectsCrossPanelHandoffs()
        {
            using var first = new PanelFixture();
            using var second = new PanelFixture();
            var source = new VisualElement();
            var destination = new VisualElement();
            first.Root.Add(source);
            second.Root.Add(destination);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            ObjectId sourceHost = Id("75300000-0000-4000-8000-000000000001");
            ObjectId destinationHost = Id("75300000-0000-4000-8000-000000000002");
            world.Install(source, sourceHost, Descriptor(sourceHost, 1));

            Assert.Throws<BattlementUiException>(() =>
            {
                world.Install(destination, destinationHost, Descriptor(destinationHost, 2));
                world.PostLayout();
            });
        }

        private static MotionDescriptor Descriptor(ObjectId host, uint generation) =>
            new(
                new ObjectId(Guid.NewGuid()),
                host,
                generation,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Controlled(Id("75200000-0000-4000-8000-000000000099")),
                ReducedMotionPolicy.Never,
                Layout: new MotionLayoutDescriptor(
                    MotionLayoutMode.Both,
                    new MotionLayoutIdentity("layout-test-group", 17),
                    new MotionLayoutIdentity("System.UInt32", 29),
                    false,
                    false,
                    false,
                    new TransitionDefinition(
                        new TransitionGenerator.Tween(
                            1_000_000,
                            new MotionEasing[] { new MotionEasing.Linear() },
                            null
                        ),
                        0,
                        new MotionRepeat.None(),
                        0,
                        MotionRepeatType.Loop
                    )
                )
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));

        private sealed class PanelFixture : IDisposable
        {
            private readonly GameObject gameObject;
            private readonly PanelSettings settings;

            public PanelFixture()
            {
                gameObject = new GameObject("Layout Projection Test Panel");
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
