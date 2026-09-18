#nullable enable

using System;
using System.Collections.Generic;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class WorldLayoutProjectionTests
    {
        [TestCase(true)]
        [TestCase(false)]
        public void CameraPlaneRectangleRoundTripsCompatibleScreenGeometry(bool orthographic)
        {
            using var fixture = new CameraFixture(orthographic);
            MotionProjectionDescriptor projection = Projection();

            ViewportRect viewport = BattlementWorldLayoutProjection.Project(
                projection,
                fixture.Camera,
                fixture.Displays
            );
            Rect result = BattlementWorldLayoutProjection.Unproject(
                viewport,
                projection,
                fixture.Camera,
                fixture.Displays
            );

            Assert.That(result.X, Is.EqualTo(projection.WorldRect.X).Within(0.001));
            Assert.That(result.Y, Is.EqualTo(projection.WorldRect.Y).Within(0.001));
            Assert.That(result.Width, Is.EqualTo(projection.WorldRect.Width).Within(0.001));
            Assert.That(result.Height, Is.EqualTo(projection.WorldRect.Height).Within(0.001));
        }

        [Test]
        public void MissingWorldProjectionRejectsBeforeTransformMutation()
        {
            var gameObject = new GameObject("Missing Projection");
            try
            {
                gameObject.transform.localPosition = new UnityEngine.Vector3(3, 4, 5);
                var target = new BattlementWorldMotionTarget(gameObject.transform);
                using var world = new BattlementMotionWorld(registerPlayerLoop: false);
                ObjectId host = Id("75300000-0000-4000-8000-000000000021");

                Assert.Throws<BattlementUiException>(() =>
                    world.Prepare(target, host, Descriptor(host, projection: null))
                );
                Assert.That(
                    gameObject.transform.localPosition,
                    Is.EqualTo(new UnityEngine.Vector3(3, 4, 5))
                );
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(gameObject);
            }
        }

        [Test]
        public void WorldProjectionComposesWithAuthoredMotionAndRestoresExactPlacement()
        {
            var gameObject = new GameObject("Projected World Host");
            try
            {
                gameObject.transform.localPosition = new UnityEngine.Vector3(4, 5, 0);
                var target = new BattlementWorldMotionTarget(gameObject.transform);

                target.SetLayoutProjection(
                    new UnityEngine.Vector3(1, 2, 0),
                    new UnityEngine.Vector3(2, 3, 1)
                );
                target.WriteScalar(MotionProperty.LocalPositionX, 6);

                Assert.That(
                    gameObject.transform.localPosition,
                    Is.EqualTo(new UnityEngine.Vector3(7, 7, 0))
                );
                Assert.That(
                    gameObject.transform.localScale,
                    Is.EqualTo(new UnityEngine.Vector3(2, 3, 1))
                );
                target.ClearLayoutProjection();
                Assert.That(
                    gameObject.transform.localPosition,
                    Is.EqualTo(new UnityEngine.Vector3(6, 5, 0))
                );
                Assert.That(gameObject.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(gameObject);
            }
        }

        private static MotionProjectionDescriptor Projection() =>
            new(
                new CameraTarget.Input(),
                new MotionProjectionPlane(
                    new Battlement.Vector3(0, 0, 0),
                    new Battlement.Vector3(1, 0, 0),
                    new Battlement.Vector3(0, 1, 0)
                ),
                new Rect(-2, -1, 4, 2)
            );

        private static MotionDescriptor Descriptor(
            ObjectId host,
            MotionProjectionDescriptor? projection
        ) =>
            new(
                new ObjectId(Guid.NewGuid()),
                host,
                1,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Controlled(Id("75300000-0000-4000-8000-000000000099")),
                ReducedMotionPolicy.Never,
                Layout: new MotionLayoutDescriptor(
                    MotionLayoutMode.Both,
                    new MotionLayoutIdentity("world-layout-test", 1),
                    new MotionLayoutIdentity("System.UInt32", 2),
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
                    ),
                    projection
                )
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));

        private sealed class CameraFixture : IDisposable
        {
            private readonly GameObject gameObject;

            public CameraFixture(bool orthographic)
            {
                gameObject = new GameObject("Layout Projection Camera");
                Camera = gameObject.AddComponent<Camera>();
                Camera.orthographic = orthographic;
                Camera.orthographicSize = 3;
                Camera.fieldOfView = 60;
                Camera.aspect = 4f / 3f;
                Camera.nearClipPlane = 0.1f;
                gameObject.transform.position = new UnityEngine.Vector3(0, 0, -10);
                Displays = new FakeDisplays();
                Displays.Set(
                    0,
                    new BattlementDisplayGeometry(
                        800,
                        600,
                        new UnityEngine.Rect(0, 0, 800, 600),
                        1,
                        null,
                        DisplayOrientation.Landscape
                    )
                );
            }

            public Camera Camera { get; }

            public FakeDisplays Displays { get; }

            public void Dispose() => UnityEngine.Object.DestroyImmediate(gameObject);
        }

        private sealed class FakeDisplays : IBattlementGeometryDisplaySource
        {
            private readonly Dictionary<DisplayId, BattlementDisplayGeometry> values = new();

            public void Set(uint id, BattlementDisplayGeometry value) =>
                values[new DisplayId(id)] = value;

            public bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry) =>
                values.TryGetValue(id, out geometry);
        }
    }
}
