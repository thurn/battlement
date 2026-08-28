#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityBounds = UnityEngine.Bounds;
using UnityObject = UnityEngine.Object;
using UnityVector3 = UnityEngine.Vector3;

namespace Battlement.Tests
{
    public sealed class BattlementWorldBoundsGeometryTests
    {
        [Test]
        public void SamplesCombinedRendererBoundsDeterministically()
        {
            var target = new GameObject("Rendered Bounds Target");
            GameObject left = Cube(
                target,
                new UnityVector3(-1.5f, 0, 6),
                new UnityVector3(2, 2, 2)
            );
            GameObject right = Cube(target, new UnityVector3(2, 1, 8), new UnityVector3(1, 3, 2));
            GameObject disabled = Cube(
                target,
                new UnityVector3(20, 0, 6),
                new UnityVector3(4, 4, 4)
            );
            disabled.GetComponent<Renderer>().enabled = false;
            Camera camera = CameraObject("Bounds Camera").GetComponent<Camera>();
            var world = new FakeWorld { InputCamera = camera };
            var displays = Displays();
            ObjectId targetId = Id(50);
            world.Set(targetId, target);
            var sampler = Sampler(world, displays, targetId);
            try
            {
                GeometryObservationBatch first = sampler.Sample()!;
                UnityBounds expected = left.GetComponent<Renderer>().bounds;
                expected.Encapsulate(right.GetComponent<Renderer>().bounds);
                AssertBounds(first, camera, expected, 800, 600);

                right.transform.SetSiblingIndex(0);

                Assert.That(sampler.Sample()!.Changed, Is.Empty);

                disabled.GetComponent<Renderer>().enabled = true;
                GeometryObservationBatch expanded = sampler.Sample()!;
                expected.Encapsulate(disabled.GetComponent<Renderer>().bounds);
                AssertBounds(expanded, camera, expected, 800, 600);
            }
            finally
            {
                UnityObject.DestroyImmediate(camera.gameObject);
                UnityObject.DestroyImmediate(target);
            }
        }

        [Test]
        public void ReportsEmptyAndDisabledRendererStatesAcrossCameraChanges()
        {
            var target = new GameObject("Changing Bounds Target");
            Camera camera = CameraObject("Changing Bounds Camera").GetComponent<Camera>();
            var world = new FakeWorld { InputCamera = camera };
            var displays = Displays();
            ObjectId targetId = Id(51);
            world.Set(targetId, target);
            var sampler = Sampler(world, displays, targetId);
            var texture = new RenderTexture(32, 32, 0);
            try
            {
                camera.targetTexture = texture;
                AssertUnavailable(sampler.Sample()!, GeometryUnavailable.NoRenderers);

                camera.targetTexture = null;
                GameObject rendered = Cube(
                    target,
                    new UnityVector3(0.5f, 0, 6),
                    new UnityVector3(2, 1, 1)
                );
                GeometryObservationBatch appeared = sampler.Sample()!;
                AssertBounds(appeared, camera, rendered.GetComponent<Renderer>().bounds, 800, 600);

                camera.transform.position = new UnityVector3(1, 0.5f, 0);
                GeometryObservationBatch cameraChanged = sampler.Sample()!;
                AssertBounds(
                    cameraChanged,
                    camera,
                    rendered.GetComponent<Renderer>().bounds,
                    800,
                    600
                );

                rendered.GetComponent<Renderer>().enabled = false;
                AssertUnavailable(sampler.Sample()!, GeometryUnavailable.NoRenderers);
            }
            finally
            {
                UnityObject.DestroyImmediate(texture);
                UnityObject.DestroyImmediate(camera.gameObject);
                UnityObject.DestroyImmediate(target);
            }
        }

        [Test]
        public void ClipsIntersectingBoundsAndRejectsBoundsBehindTheNearPlane()
        {
            var target = new GameObject("Near Plane Bounds Target");
            GameObject rendered = Cube(
                target,
                new UnityVector3(0, 0, 1),
                new UnityVector3(2, 2, 2)
            );
            Camera camera = CameraObject("Near Plane Bounds Camera").GetComponent<Camera>();
            camera.nearClipPlane = 1;
            var world = new FakeWorld { InputCamera = camera };
            ObjectId targetId = Id(52);
            world.Set(targetId, target);
            var sampler = Sampler(world, Displays(), targetId);
            try
            {
                WorldBoundsGeometry clipped = Current(sampler.Sample()!);
                Assert.That(clipped.NearestDepth, Is.EqualTo(1).Within(0.0001));
                Assert.That(clipped.FarthestDepth, Is.EqualTo(2).Within(0.0001));
                Assert.That(clipped.Bound.X, Is.LessThan(0));
                Assert.That(clipped.Bound.Width, Is.GreaterThan(800));
                Assert.That(clipped.IsInsideViewport, Is.True);

                rendered.transform.position = UnityVector3.zero;
                rendered.transform.localScale = UnityVector3.one * 0.5f;
                camera.projectionMatrix = new Matrix4x4();

                AssertUnavailable(sampler.Sample()!, GeometryUnavailable.BehindCamera);
            }
            finally
            {
                UnityObject.DestroyImmediate(camera.gameObject);
                UnityObject.DestroyImmediate(target);
            }
        }

        [Test]
        public void RejectsProjectionHorizonCrossingTheClippedBounds()
        {
            var target = new GameObject("Horizon Bounds Target");
            Cube(target, new UnityVector3(0, 0, 5), new UnityVector3(2, 2, 2));
            Camera camera = CameraObject("Horizon Bounds Camera").GetComponent<Camera>();
            Matrix4x4 projection = camera.projectionMatrix;
            projection[3, 0] = 10;
            camera.projectionMatrix = projection;
            Assert.That(projection.determinant, Is.Not.Zero);
            var world = new FakeWorld { InputCamera = camera };
            ObjectId targetId = Id(53);
            world.Set(targetId, target);
            var sampler = Sampler(world, Displays(), targetId);
            try
            {
                AssertUnavailable(sampler.Sample()!, GeometryUnavailable.ProjectionUnavailable);
            }
            finally
            {
                UnityObject.DestroyImmediate(camera.gameObject);
                UnityObject.DestroyImmediate(target);
            }
        }

        [Test]
        public void IncludesLeftAndTopViewportContactButExcludesRightAndBottomContact()
        {
            var target = new GameObject("Viewport Edge Bounds Target");
            GameObject rendered = Cube(target, UnityVector3.zero, new UnityVector3(1, 1, 0));
            Camera camera = CameraObject("Viewport Edge Bounds Camera").GetComponent<Camera>();
            camera.orthographic = true;
            camera.orthographicSize = 5;
            var world = new FakeWorld { InputCamera = camera };
            ObjectId targetId = Id(54);
            world.Set(targetId, target);
            var sampler = Sampler(world, Displays(), targetId);
            try
            {
                float horizontalEdge = camera.orthographicSize * camera.aspect;
                rendered.transform.position = new UnityVector3(-horizontalEdge - 0.5f, 0, 5);
                WorldBoundsGeometry left = Current(sampler.Sample()!);
                Assert.That(left.Bound.X + left.Bound.Width, Is.EqualTo(0).Within(0.001));
                Assert.That(left.IsInsideViewport, Is.True);

                rendered.transform.position = new UnityVector3(0, 5.5f, 5);
                WorldBoundsGeometry top = Current(sampler.Sample()!);
                Assert.That(top.Bound.Y + top.Bound.Height, Is.EqualTo(0).Within(0.001));
                Assert.That(top.IsInsideViewport, Is.True);

                rendered.transform.position = new UnityVector3(horizontalEdge + 0.5f, 0, 5);
                WorldBoundsGeometry right = Current(sampler.Sample()!);
                Assert.That(right.Bound.X, Is.EqualTo(800).Within(0.001));
                Assert.That(right.IsInsideViewport, Is.False);

                rendered.transform.position = new UnityVector3(0, -5.5f, 5);
                WorldBoundsGeometry bottom = Current(sampler.Sample()!);
                Assert.That(bottom.Bound.Y, Is.EqualTo(600).Within(0.001));
                Assert.That(bottom.IsInsideViewport, Is.False);
            }
            finally
            {
                UnityObject.DestroyImmediate(camera.gameObject);
                UnityObject.DestroyImmediate(target);
            }
        }

        private static BattlementGeometrySampler Sampler(
            FakeWorld world,
            FakeDisplays displays,
            ObjectId targetId
        )
        {
            var sampler = new BattlementGeometrySampler(
                new Battlement.UI.BattlementUiDocuments(),
                displays,
                world: world
            );
            sampler.Apply(
                new GeometryObservationUpdate(
                    new[]
                    {
                        new GeometryObservation(
                            ObservationId(),
                            new GeometryObservationTarget.WorldRenderedBounds(
                                targetId,
                                new CameraTarget.Input()
                            )
                        ),
                    },
                    Array.Empty<GeometryObservationId>()
                )
            );
            return sampler;
        }

        private static GameObject Cube(GameObject parent, UnityVector3 position, UnityVector3 scale)
        {
            GameObject cube = GameObject.CreatePrimitive(PrimitiveType.Cube);
            cube.transform.SetParent(parent.transform, false);
            cube.transform.position = position;
            cube.transform.localScale = scale;
            return cube;
        }

        private static GameObject CameraObject(string name)
        {
            var gameObject = new GameObject(name);
            Camera camera = gameObject.AddComponent<Camera>();
            camera.aspect = 4f / 3f;
            return gameObject;
        }

        private static FakeDisplays Displays()
        {
            var displays = new FakeDisplays();
            displays.Set(
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
            return displays;
        }

        private static void AssertBounds(
            GeometryObservationBatch batch,
            Camera camera,
            UnityBounds bounds,
            double width,
            double height
        )
        {
            WorldBoundsGeometry actual = Current(batch);
            UnityVector3[] corners = Corners(bounds);
            UnityVector3[] projected = corners.Select(camera.WorldToViewportPoint).ToArray();
            double left = projected.Min(point => point.x) * width;
            double right = projected.Max(point => point.x) * width;
            double top = height - projected.Max(point => point.y) * height;
            double bottom = height - projected.Min(point => point.y) * height;
            Assert.That(actual.Bound.X, Is.EqualTo(left).Within(0.001));
            Assert.That(actual.Bound.Y, Is.EqualTo(top).Within(0.001));
            Assert.That(actual.Bound.Width, Is.EqualTo(right - left).Within(0.001));
            Assert.That(actual.Bound.Height, Is.EqualTo(bottom - top).Within(0.001));
            Assert.That(
                actual.NearestDepth,
                Is.EqualTo(corners.Min(point => point.z - camera.transform.position.z))
                    .Within(0.001)
            );
            Assert.That(
                actual.FarthestDepth,
                Is.EqualTo(corners.Max(point => point.z - camera.transform.position.z))
                    .Within(0.001)
            );
            Assert.That(actual.IsInsideViewport, Is.True);
        }

        private static UnityVector3[] Corners(UnityBounds bounds)
        {
            UnityVector3 min = bounds.min;
            UnityVector3 max = bounds.max;
            return new[]
            {
                new UnityVector3(min.x, min.y, min.z),
                new UnityVector3(max.x, min.y, min.z),
                new UnityVector3(min.x, max.y, min.z),
                new UnityVector3(max.x, max.y, min.z),
                new UnityVector3(min.x, min.y, max.z),
                new UnityVector3(max.x, min.y, max.z),
                new UnityVector3(min.x, max.y, max.z),
                new UnityVector3(max.x, max.y, max.z),
            };
        }

        private static WorldBoundsGeometry Current(GeometryObservationBatch batch) =>
            (
                (GeometryValue.WorldBounds)
                    ((GeometryObservationResult.Current)batch.Changed.Single().Result).Value
            ).Value;

        private static void AssertUnavailable(
            GeometryObservationBatch batch,
            GeometryUnavailable expected
        ) =>
            Assert.That(
                ((GeometryObservationResult.Unavailable)batch.Changed.Single().Result).Reason,
                Is.EqualTo(expected)
            );

        private static ObjectId Id(int value) => new(new Guid(value, 0, 0, new byte[8]));

        private static GeometryObservationId ObservationId() => new(new Guid(1, 0, 0, new byte[8]));

        private sealed class FakeDisplays : IBattlementGeometryDisplaySource
        {
            private readonly Dictionary<DisplayId, BattlementDisplayGeometry> values = new();

            public void Set(uint id, BattlementDisplayGeometry value) =>
                values[new DisplayId(id)] = value;

            public bool TryGet(DisplayId id, out BattlementDisplayGeometry geometry) =>
                values.TryGetValue(id, out geometry);
        }

        private sealed class FakeWorld : IBattlementGeometryWorldSource
        {
            private readonly Dictionary<ObjectId, GameObject> values = new();

            public Camera? InputCamera { get; set; }

            public void Set(ObjectId id, GameObject value) => values[id] = value;

            public BattlementGeometryObjectKind LookupObject(
                ObjectId id,
                out GameObject? gameObject
            ) =>
                values.TryGetValue(id, out gameObject)
                    ? BattlementGeometryObjectKind.World
                    : BattlementGeometryObjectKind.Missing;
        }
    }
}
