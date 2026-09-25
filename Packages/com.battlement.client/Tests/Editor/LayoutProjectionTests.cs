#nullable enable

using System;
using System.Collections.Generic;
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
        public void SharedLayoutAcceptsCrossPanelHandoffsOnOnePhysicalDisplay()
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

            Assert.DoesNotThrow(() =>
            {
                world.Install(destination, destinationHost, Descriptor(destinationHost, 2));
                world.PostLayout();
            });
        }

        [Test]
        public void UiProjectionSpaceConvertsThroughPhysicalPixelsAcrossPanelScales()
        {
            var first = new BattlementUiProjectionSpace(new DisplayId(0), 2);
            var second = new BattlementUiProjectionSpace(new DisplayId(0), 1.25);

            ViewportRect viewport = first.ToViewport(new UnityEngine.Rect(10, 20, 100, 50));
            UnityEngine.Rect destination = second.FromViewport(viewport);

            Assert.That(destination, Is.EqualTo(new UnityEngine.Rect(16, 32, 160, 80)));
        }

        [Test]
        public void BlockingLayoutOperationFollowsRetargetedProjectionUntilArrival()
        {
            ObjectId host = Id("75300000-0000-4000-8000-000000000031");
            ObjectId descriptorId = Id("75300000-0000-4000-8000-000000000032");
            var target = new FakeLayoutTarget();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            MotionDescriptor initial = Descriptor(host, 1) with
            {
                DescriptorId = descriptorId,
                Layout = Descriptor(host, 1).Layout! with
                {
                    Projection = new MotionProjectionDescriptor(
                        new CameraTarget.Input(),
                        new MotionProjectionPlane(
                            new Battlement.Vector3(0, 0, 0),
                            new Battlement.Vector3(1, 0, 0),
                            new Battlement.Vector3(0, 1, 0)
                        ),
                        new Battlement.Rect(0, 0, 1, 1)
                    ),
                },
            };
            IBattlementCommandOperation running = world.Prepare(target, host, initial)!.Commit()!;

            Assert.That(running.IsComplete(TimeSpan.Zero), Is.False);
            IBattlementCommandOperation retargeted = world
                .Prepare(target, host, initial with { Generation = 2 })!
                .Commit()!;
            Assert.That(target.Projections[0].IsComplete, Is.True);
            Assert.That(running.IsComplete(TimeSpan.Zero), Is.False);

            target.Projections[1].Complete();

            Assert.That(running.IsComplete(TimeSpan.Zero), Is.True);
            Assert.That(retargeted.IsComplete(TimeSpan.Zero), Is.True);
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

        private sealed class FakeLayoutTarget
            : IBattlementMotionTarget,
                IBattlementLayoutProjectionTarget
        {
            public List<FakeLayoutProjection> Projections { get; } = new();

            public BattlementLayoutDomain Domain => BattlementLayoutDomain.World;

            public ViewportRect VisibleBounds(MotionLayoutDescriptor descriptor) =>
                new(0, 0, 100, 100, new DisplayId(0));

            public IBattlementLayoutProjection CreateProjection(
                MotionLayoutDescriptor descriptor,
                BattlementLayoutOrigin origin,
                ulong anchorMicros
            )
            {
                var projection = new FakeLayoutProjection(descriptor, origin.Bounds);
                Projections.Add(projection);
                return projection;
            }

            public bool Supports(MotionProperty property) => true;

            public bool IsLayout(MotionProperty property) => false;

            public bool IsSpatial(MotionProperty property) => false;

            public MotionValue Read(MotionProperty property) => new MotionValue.Scalar(0);

            public void Write(MotionProperty property, MotionValue value) { }

            public void WriteScalar(MotionProperty property, double value) { }

            public void WriteAdaptedScalar(MotionProperty property, double value) { }

            public void SetContribution(MotionProperty property, MotionValue value) { }

            public void RemoveContribution(MotionProperty property) { }

            public bool Contains(IBattlementMotionTarget target) => ReferenceEquals(this, target);

            public bool IsParentOf(IBattlementMotionTarget target) => false;

            public IReadOnlyList<MotionPropertyValue> ResolvePosition(
                IBattlementMotionTarget reference,
                string? anchor,
                Battlement.Vector3 offset
            ) => Array.Empty<MotionPropertyValue>();

            public void Release() { }
        }

        private sealed class FakeLayoutProjection : IBattlementLayoutProjection
        {
            public FakeLayoutProjection(MotionLayoutDescriptor descriptor, ViewportRect bounds) =>
                (Descriptor, VisibleBounds) = (descriptor, bounds);

            public MotionLayoutDescriptor Descriptor { get; }

            public BattlementLayoutDomain Domain => BattlementLayoutDomain.World;

            public ViewportRect VisibleBounds { get; }

            public bool IsComplete { get; private set; }

            public bool IsPaused { get; private set; }

            public void CaptureDestination() { }

            public void Sample(ulong clockMicros, bool reducedMotion = false) { }

            public void Pause(ulong clockMicros) => IsPaused = true;

            public void Resume(ulong clockMicros) => IsPaused = false;

            public void Complete() => IsComplete = true;

            public void Release() => IsComplete = true;

            public void Abort() => IsComplete = true;
        }
    }
}
