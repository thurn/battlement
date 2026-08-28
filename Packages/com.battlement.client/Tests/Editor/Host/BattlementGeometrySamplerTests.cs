#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiBox = Battlement.UiElement.Box;

namespace Battlement.Tests
{
    public sealed class BattlementGeometrySamplerTests
    {
        [UnityTest]
        public IEnumerator SamplesScaledElementViewportAndAvailabilityInOnePass()
        {
            ObjectId documentId = Id(1);
            ObjectId rootId = Id(2);
            ObjectId elementId = Id(3);
            ObjectId detachedId = Id(4);
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(
                    rootId,
                    new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize, Scale: 2)
                )
            );
            var documents = new BattlementUiDocuments();
            var displays = new FakeDisplays();
            displays.Set(
                0,
                new BattlementDisplayGeometry(
                    800,
                    600,
                    new UnityEngine.Rect(10, 20, 780, 550),
                    1.25,
                    144,
                    DisplayOrientation.Landscape
                )
            );
            displays.Set(
                1,
                new BattlementDisplayGeometry(
                    1024,
                    768,
                    new UnityEngine.Rect(0, 0, 1024, 768),
                    1,
                    null,
                    DisplayOrientation.LandscapeFlipped
                )
            );
            try
            {
                documents.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            Children: new[]
                            {
                                new UiNode(
                                    elementId,
                                    new UiBox
                                    {
                                        Style = new UiStyle(
                                            Position: UiStyle.Set(UiPosition.Absolute),
                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(12)
                                            ),
                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(18)
                                            ),
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(40)
                                            ),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(30)
                                            )
                                        ),
                                    }
                                ),
                            }
                        ),
                    },
                    id => id == documentId ? owned : null
                );
                yield return null;
                yield return null;

                var sampler = new BattlementGeometrySampler(documents, displays);
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(10, new GeometryObservationTarget.UiElement(elementId)),
                            Observation(11, new GeometryObservationTarget.UiElement(detachedId)),
                            Observation(
                                12,
                                new GeometryObservationTarget.Viewport(new DisplayId(0))
                            ),
                            Observation(
                                13,
                                new GeometryObservationTarget.Viewport(new DisplayId(1))
                            ),
                            Observation(
                                14,
                                new GeometryObservationTarget.Viewport(new DisplayId(2))
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );

                GeometryObservationBatch first = sampler.Sample()!;
                Assert.That(first.Generation.Value, Is.EqualTo(1));
                Assert.That(first.Changed, Has.Count.EqualTo(5));
                var element = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(first, 10).Result).Value;
                Assert.That(element.Value.Layout, Is.EqualTo(new Battlement.Rect(12, 18, 40, 30)));
                Assert.That(element.Value.ViewportBound.DisplayId, Is.EqualTo(new DisplayId(0)));
                Assert.That(element.Value.ViewportBound.Width, Is.EqualTo(80).Within(0.01));
                Assert.That(element.Value.ViewportBound.Height, Is.EqualTo(60).Within(0.01));
                Assert.That(element.Value.ViewportFromLocal.M11, Is.EqualTo(2).Within(0.01));
                Assert.That(element.Value.ViewportFromLocal.M22, Is.EqualTo(2).Within(0.01));
                Assert.That(element.Value.PanelId, Is.EqualTo(rootId));

                var viewport = (GeometryValue.Viewport)
                    ((GeometryObservationResult.Current)Value(first, 12).Result).Value;
                Assert.That(
                    viewport.Value.Viewport,
                    Is.EqualTo(new ViewportRect(0, 0, 800, 600, new DisplayId(0)))
                );
                Assert.That(
                    viewport.Value.SafeArea,
                    Is.EqualTo(new ViewportRect(10, 30, 780, 550, new DisplayId(0)))
                );
                Assert.That(viewport.Value.Scale, Is.EqualTo(1.25));
                Assert.That(viewport.Value.Dpi, Is.EqualTo(144));
                AssertUnavailable(first, 11, GeometryUnavailable.Detached);
                AssertUnavailable(first, 14, GeometryUnavailable.DisplayUnavailable);
                Assert.That(sampler.Sample()!.Changed, Is.Empty);

                displays.Set(
                    0,
                    new BattlementDisplayGeometry(
                        800,
                        600,
                        new UnityEngine.Rect(20, 40, 760, 520),
                        1.25,
                        144,
                        DisplayOrientation.Landscape
                    )
                );
                GeometryObservationBatch safeAreaChange = sampler.Sample()!;
                Assert.That(safeAreaChange.Generation.Value, Is.EqualTo(3));
                Assert.That(safeAreaChange.Changed, Has.Count.EqualTo(1));
                Assert.That(safeAreaChange.Changed[0].ObservationId, Is.EqualTo(ObservationId(12)));

                Assert.That(documents.TryGet(elementId, out VisualElement? target), Is.True);
                target!.style.display = DisplayStyle.None;
                yield return null;
                GeometryObservationBatch hidden = sampler.Sample()!;
                Assert.That(hidden.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(hidden, 10, GeometryUnavailable.Hidden);

                target.style.display = DisplayStyle.Flex;
                target.style.scale = new Scale(new UnityEngine.Vector3(0, 1, 1));
                yield return null;
                GeometryObservationBatch singular = sampler.Sample()!;
                Assert.That(singular.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(singular, 10, GeometryUnavailable.ProjectionUnavailable);

                target.style.scale = new Scale(UnityEngine.Vector3.one);
                UnityEngine.UIElements.PanelSettings panel = owned
                    .GetComponent<UIDocument>()
                    .panelSettings;
                panel.targetDisplay = 1;
                yield return null;
                GeometryObservationBatch mapped = sampler.Sample()!;
                Assert.That(mapped.Changed, Has.Count.EqualTo(1));
                var mappedElement = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(mapped, 10).Result).Value;
                Assert.That(
                    mappedElement.Value.ViewportBound.DisplayId,
                    Is.EqualTo(new DisplayId(1))
                );
                Assert.That(
                    mappedElement.Value.ViewportFromLocal.M11,
                    Is.EqualTo(target.panel.scaledPixelsPerPoint).Within(0.01)
                );

                panel.targetDisplay = 7;
                yield return null;
                GeometryObservationBatch missingDisplay = sampler.Sample()!;
                Assert.That(missingDisplay.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(missingDisplay, 10, GeometryUnavailable.DisplayUnavailable);
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        [UnityTest]
        public IEnumerator ProjectsWorldPanelsClipsNearPlaneAndRejectsUnmappedPanels()
        {
            ObjectId worldDocumentId = Id(20);
            ObjectId worldRootId = Id(21);
            ObjectId textureDocumentId = Id(22);
            ObjectId textureRootId = Id(23);
            GameObject worldObject = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(
                    worldRootId,
                    new PanelSettingsValue(
                        RenderMode: PanelRenderMode.WorldSpace,
                        ScaleMode: PanelScaleMode.ConstantPixelSize
                    ),
                    DocumentPosition.Absolute,
                    WorldSpaceSizeMode.Fixed,
                    new ScreenSize(100, 60),
                    PivotReferenceSize.Layout
                )
            );
            GameObject textureObject = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(
                    textureRootId,
                    new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize)
                )
            );
            var cameraObject = new GameObject("Geometry Camera");
            Camera camera = cameraObject.AddComponent<Camera>();
            var texture = new RenderTexture(64, 64, 0);
            var documents = new BattlementUiDocuments();
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
            displays.Set(
                1,
                new BattlementDisplayGeometry(
                    1000,
                    800,
                    new UnityEngine.Rect(0, 0, 1000, 800),
                    1,
                    null,
                    DisplayOrientation.Landscape
                )
            );
            try
            {
                textureObject.GetComponent<UIDocument>().panelSettings.targetTexture = texture;
                camera.nearClipPlane = 0.3f;
                camera.farClipPlane = 100;
                camera.fieldOfView = 60;
                camera.aspect = 1.25f;
                camera.targetDisplay = 1;
                camera.rect = new UnityEngine.Rect(0.1f, 0.2f, 0.8f, 0.6f);
                worldObject.transform.position = new UnityEngine.Vector3(0, 0, 4);
                documents.Replace(
                    new[]
                    {
                        WorldDocument(worldDocumentId, worldRootId),
                        WorldDocument(textureDocumentId, textureRootId),
                    },
                    id =>
                        id == worldDocumentId ? worldObject
                        : id == textureDocumentId ? textureObject
                        : null
                );
                yield return null;
                yield return null;

                var sampler = new BattlementGeometrySampler(documents, displays, () => camera);
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(20, new GeometryObservationTarget.UiElement(worldRootId)),
                            Observation(21, new GeometryObservationTarget.UiElement(textureRootId)),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );

                GeometryObservationBatch first = sampler.Sample()!;
                Assert.That(first.Changed, Has.Count.EqualTo(2));
                var projected = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(first, 20).Result).Value;
                Assert.That(projected.Value.ViewportBound.DisplayId, Is.EqualTo(new DisplayId(1)));
                Assert.That(projected.Value.ViewportBound.Width, Is.GreaterThan(0));
                Assert.That(projected.Value.ViewportBound.Height, Is.GreaterThan(0));
                AssertUnavailable(first, 21, GeometryUnavailable.NoViewportMapping);

                Assert.That(documents.TryGet(worldRootId, out VisualElement? root), Is.True);
                UnityEngine.Vector2 localCenter = root!.layout.center;
                UnityEngine.Vector3 worldCenter = worldObject.transform.TransformPoint(
                    root.worldTransform.MultiplyPoint3x4(localCenter)
                );
                UnityEngine.Vector3 viewportCenter = camera.WorldToViewportPoint(worldCenter);
                UnityEngine.Vector2 expectedCenter = new(
                    (camera.rect.x + viewportCenter.x * camera.rect.width) * 1000,
                    800 - (camera.rect.y + viewportCenter.y * camera.rect.height) * 800
                );
                Assert.That(
                    Map(projected.Value.ViewportFromLocal, localCenter).x,
                    Is.EqualTo(expectedCenter.x).Within(0.1)
                );
                Assert.That(
                    Map(projected.Value.ViewportFromLocal, localCenter).y,
                    Is.EqualTo(expectedCenter.y).Within(0.1)
                );

                UnityEngine.Vector3 localLeft = root.worldTransform.MultiplyPoint3x4(
                    UnityEngine.Vector3.zero
                );
                UnityEngine.Vector3 localRight = root.worldTransform.MultiplyPoint3x4(
                    new UnityEngine.Vector3(root.layout.width, 0, 0)
                );
                float width = UnityEngine.Vector3.Distance(localLeft, localRight);
                worldObject.transform.localScale = UnityEngine.Vector3.one / width;
                worldObject.transform.SetPositionAndRotation(
                    new UnityEngine.Vector3(0, 0, 0.55f),
                    UnityEngine.Quaternion.Euler(0, 60, 0)
                );
                yield return null;

                double[] depths = Corners(root)
                    .Select(point =>
                        (double)
                            UnityEngine.Vector3.Dot(
                                camera.transform.forward,
                                worldObject.transform.TransformPoint(point)
                                    - camera.transform.position
                            )
                    )
                    .ToArray();
                Assert.That(depths.Min(), Is.LessThan(camera.nearClipPlane));
                Assert.That(depths.Max(), Is.GreaterThan(camera.nearClipPlane));
                GeometryObservationBatch clipped = sampler.Sample()!;
                var clippedValue = (GeometryValue.Element)
                    ((GeometryObservationResult.Current)Value(clipped, 20).Result).Value;
                UnityEngine.Rect expectedClipped = ExpectedClippedBound(
                    camera,
                    worldObject.transform,
                    root,
                    1000,
                    800
                );
                Assert.That(
                    clippedValue.Value.ViewportBound.X,
                    Is.EqualTo(expectedClipped.x).Within(0.1)
                );
                Assert.That(
                    clippedValue.Value.ViewportBound.Y,
                    Is.EqualTo(expectedClipped.y).Within(0.1)
                );
                Assert.That(
                    clippedValue.Value.ViewportBound.Width,
                    Is.EqualTo(expectedClipped.width).Within(0.1)
                );
                Assert.That(
                    clippedValue.Value.ViewportBound.Height,
                    Is.EqualTo(expectedClipped.height).Within(0.1)
                );

                camera.enabled = false;
                GeometryObservationBatch unavailable = sampler.Sample()!;
                Assert.That(unavailable.Changed, Has.Count.EqualTo(1));
                AssertUnavailable(unavailable, 20, GeometryUnavailable.CameraDisabled);

                camera.enabled = true;
                worldObject.transform.SetPositionAndRotation(
                    new UnityEngine.Vector3(0, 0, 4),
                    UnityEngine.Quaternion.identity
                );
                Matrix4x4 horizonProjection = camera.projectionMatrix;
                horizonProjection.m30 = 100;
                camera.projectionMatrix = horizonProjection;
                yield return null;
                GeometryObservationBatch horizon = sampler.Sample()!;
                AssertUnavailable(horizon, 20, GeometryUnavailable.ProjectionUnavailable);

                camera.ResetProjectionMatrix();
                displays.Set(
                    1,
                    new BattlementDisplayGeometry(
                        double.NaN,
                        800,
                        new UnityEngine.Rect(0, 0, 1000, 800),
                        1,
                        null,
                        DisplayOrientation.Landscape
                    )
                );
                Assert.Throws<InvalidOperationException>(() => sampler.Sample());
            }
            finally
            {
                documents.Clear();
                Object.DestroyImmediate(texture);
                Object.DestroyImmediate(cameraObject);
                Object.DestroyImmediate(textureObject);
                Object.DestroyImmediate(worldObject);
            }
        }

        [Test]
        public void SamplesWorldOriginsAndNamedAnchorsAcrossCameraTargets()
        {
            ObjectId targetId = Id(30);
            ObjectId cameraId = Id(31);
            ObjectId missingId = Id(32);
            var target = new GameObject("Geometry Target");
            var anchorObject = new GameObject("Head Anchor");
            anchorObject.transform.SetParent(target.transform, false);
            anchorObject.transform.localPosition = new UnityEngine.Vector3(0, 1, 0);
            anchorObject.AddComponent<BattlementGeometryAnchor>().Name = "head";
            BattlementGeometryAnchorMap.Attach(
                target,
                BattlementGeometryAnchorCatalog.Capture(target)
            );
            var inputCameraObject = new GameObject("Input Geometry Camera");
            Camera inputCamera = inputCameraObject.AddComponent<Camera>();
            var explicitCameraObject = new GameObject("Explicit Geometry Camera");
            Camera explicitCamera = explicitCameraObject.AddComponent<Camera>();
            var world = new FakeWorld { InputCamera = inputCamera };
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
            displays.Set(
                1,
                new BattlementDisplayGeometry(
                    1000,
                    800,
                    new UnityEngine.Rect(0, 0, 1000, 800),
                    1,
                    null,
                    DisplayOrientation.Landscape
                )
            );
            try
            {
                target.transform.position = new UnityEngine.Vector3(0.5f, -0.25f, 5);
                inputCamera.aspect = 4f / 3f;
                explicitCamera.aspect = 5f / 4f;
                explicitCamera.targetDisplay = 1;
                explicitCamera.rect = new UnityEngine.Rect(0.1f, 0.2f, 0.8f, 0.6f);
                explicitCameraObject.transform.position = new UnityEngine.Vector3(1, 0, 0);
                world.Set(targetId, target);
                world.Set(cameraId, explicitCameraObject);
                var sampler = new BattlementGeometrySampler(
                    new BattlementUiDocuments(),
                    displays,
                    world: world
                );
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(
                                30,
                                new GeometryObservationTarget.WorldOrigin(
                                    targetId,
                                    new CameraTarget.Input()
                                )
                            ),
                            Observation(
                                31,
                                new GeometryObservationTarget.WorldAnchor(
                                    targetId,
                                    new AnchorName("head"),
                                    new CameraTarget.Object(cameraId)
                                )
                            ),
                            Observation(
                                32,
                                new GeometryObservationTarget.WorldOrigin(
                                    missingId,
                                    new CameraTarget.Input()
                                )
                            ),
                            Observation(
                                33,
                                new GeometryObservationTarget.WorldOrigin(
                                    targetId,
                                    new CameraTarget.Object(missingId)
                                )
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );

                GeometryObservationBatch first = sampler.Sample()!;
                AssertWorldPoint(first, 30, inputCamera, target.transform, 800, 600, 0);
                AssertWorldPoint(first, 31, explicitCamera, anchorObject.transform, 1000, 800, 1);
                AssertUnavailable(first, 32, GeometryUnavailable.ObjectMissing);
                AssertUnavailable(first, 33, GeometryUnavailable.CameraDisabled);

                target.SetActive(false);
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(
                                34,
                                new GeometryObservationTarget.WorldOrigin(
                                    targetId,
                                    new CameraTarget.Input()
                                )
                            ),
                            Observation(
                                35,
                                new GeometryObservationTarget.WorldAnchor(
                                    targetId,
                                    new AnchorName("head"),
                                    new CameraTarget.Object(cameraId)
                                )
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );
                GeometryObservationBatch inactive = sampler.Sample()!;
                AssertWorldPoint(inactive, 34, inputCamera, target.transform, 800, 600, 0);
                AssertWorldPoint(
                    inactive,
                    35,
                    explicitCamera,
                    anchorObject.transform,
                    1000,
                    800,
                    1
                );

                inputCamera.enabled = false;
                explicitCamera.enabled = false;
                GeometryObservationBatch inactiveCamera = sampler.Sample()!;
                AssertUnavailable(inactiveCamera, 34, GeometryUnavailable.CameraDisabled);
                AssertUnavailable(inactiveCamera, 35, GeometryUnavailable.CameraDisabled);

                target.SetActive(true);
                inputCamera.enabled = true;
                explicitCamera.enabled = true;
                target.transform.position = new UnityEngine.Vector3(0, 0, -2);
                GeometryObservationBatch behind = sampler.Sample()!;
                AssertUnavailable(behind, 30, GeometryUnavailable.BehindCamera);
                AssertUnavailable(behind, 31, GeometryUnavailable.BehindCamera);

                target.transform.position = new UnityEngine.Vector3(0, 0, 5);
                explicitCamera.enabled = false;
                GeometryObservationBatch disabled = sampler.Sample()!;
                AssertUnavailable(disabled, 31, GeometryUnavailable.CameraDisabled);

                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(
                                36,
                                new GeometryObservationTarget.WorldAnchor(
                                    targetId,
                                    new AnchorName("missing"),
                                    new CameraTarget.Input()
                                )
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );
                Assert.Throws<InvalidOperationException>(() => sampler.Sample());
            }
            finally
            {
                Object.DestroyImmediate(explicitCameraObject);
                Object.DestroyImmediate(inputCameraObject);
                Object.DestroyImmediate(target);
            }
        }

        [Test]
        public void WorldSamplingCommitsItsCacheOnlyAfterACompletePass()
        {
            ObjectId targetId = Id(40);
            var target = new GameObject("Atomic Geometry Target");
            var cameraObject = new GameObject("Atomic Geometry Camera");
            Camera camera = cameraObject.AddComponent<Camera>();
            var world = new FakeWorld { InputCamera = camera };
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
            try
            {
                target.transform.position = new UnityEngine.Vector3(0, 0, 5);
                BattlementGeometryAnchorMap.Attach(
                    target,
                    BattlementGeometryAnchorCatalog.Capture(target)
                );
                world.Set(targetId, target);
                var sampler = new BattlementGeometrySampler(
                    new BattlementUiDocuments(),
                    displays,
                    world: world
                );
                sampler.Apply(
                    new GeometryObservationUpdate(
                        new[]
                        {
                            Observation(
                                40,
                                new GeometryObservationTarget.WorldOrigin(
                                    targetId,
                                    new CameraTarget.Input()
                                )
                            ),
                            Observation(
                                41,
                                new GeometryObservationTarget.WorldAnchor(
                                    targetId,
                                    new AnchorName("missing"),
                                    new CameraTarget.Input()
                                )
                            ),
                        },
                        Array.Empty<GeometryObservationId>()
                    )
                );

                Assert.Throws<InvalidOperationException>(() => sampler.Sample());
                sampler.Apply(
                    new GeometryObservationUpdate(
                        Array.Empty<GeometryObservation>(),
                        new[] { ObservationId(41) }
                    )
                );

                GeometryObservationBatch recovered = sampler.Sample()!;
                AssertWorldPoint(recovered, 40, camera, target.transform, 800, 600, 0);
            }
            finally
            {
                Object.DestroyImmediate(cameraObject);
                Object.DestroyImmediate(target);
            }
        }

        [Test]
        public void WorldSamplingRejectsUiObjectIdentities()
        {
            ObjectId uiId = Id(42);
            var world = new FakeWorld();
            world.SetUi(uiId);
            var sampler = new BattlementGeometrySampler(new BattlementUiDocuments(), world: world);
            sampler.Apply(
                new GeometryObservationUpdate(
                    new[]
                    {
                        Observation(
                            42,
                            new GeometryObservationTarget.WorldOrigin(
                                uiId,
                                new CameraTarget.Input()
                            )
                        ),
                    },
                    Array.Empty<GeometryObservationId>()
                )
            );

            Assert.Throws<InvalidOperationException>(() => sampler.Sample());
        }

        private static UiDocument WorldDocument(ObjectId documentId, ObjectId rootId) =>
            new(
                documentId,
                rootId,
                Style: new UiStyle(
                    Width: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(100)),
                    Height: UiStyle.Set<UiLengthOrAuto>(new UiLengthOrAuto.Px(60))
                )
            );

        private static IEnumerable<UnityEngine.Vector3> Corners(VisualElement element)
        {
            yield return element.worldTransform.MultiplyPoint3x4(UnityEngine.Vector3.zero);
            yield return element.worldTransform.MultiplyPoint3x4(
                new UnityEngine.Vector3(element.layout.width, 0, 0)
            );
            yield return element.worldTransform.MultiplyPoint3x4(
                new UnityEngine.Vector3(element.layout.width, element.layout.height, 0)
            );
            yield return element.worldTransform.MultiplyPoint3x4(
                new UnityEngine.Vector3(0, element.layout.height, 0)
            );
        }

        private static UnityEngine.Rect ExpectedClippedBound(
            Camera camera,
            Transform document,
            VisualElement element,
            float displayWidth,
            float displayHeight
        )
        {
            var input = Corners(element)
                .Select(document.TransformPoint)
                .Select(point =>
                    (
                        Point: point,
                        Depth: UnityEngine.Vector3.Dot(
                            camera.transform.forward,
                            point - camera.transform.position
                        )
                    )
                )
                .ToList();
            var clipped = new List<(UnityEngine.Vector3 Point, float Depth)>(input.Count + 2);
            (UnityEngine.Vector3 Point, float Depth) previous = input[^1];
            bool previousInside = previous.Depth >= camera.nearClipPlane;
            foreach ((UnityEngine.Vector3 Point, float Depth) current in input)
            {
                bool currentInside = current.Depth >= camera.nearClipPlane;
                if (currentInside != previousInside)
                {
                    float amount =
                        (camera.nearClipPlane - previous.Depth) / (current.Depth - previous.Depth);
                    clipped.Add(
                        (
                            UnityEngine.Vector3.LerpUnclamped(
                                previous.Point,
                                current.Point,
                                amount
                            ),
                            camera.nearClipPlane
                        )
                    );
                }
                if (currentInside)
                    clipped.Add(current);
                previous = current;
                previousInside = currentInside;
            }

            float minX = float.PositiveInfinity;
            float minY = float.PositiveInfinity;
            float maxX = float.NegativeInfinity;
            float maxY = float.NegativeInfinity;
            foreach ((UnityEngine.Vector3 point, _) in clipped)
            {
                UnityEngine.Vector3 viewport = camera.WorldToViewportPoint(point);
                float x = (camera.rect.x + viewport.x * camera.rect.width) * displayWidth;
                float y =
                    displayHeight
                    - (camera.rect.y + viewport.y * camera.rect.height) * displayHeight;
                minX = Mathf.Min(minX, x);
                minY = Mathf.Min(minY, y);
                maxX = Mathf.Max(maxX, x);
                maxY = Mathf.Max(maxY, y);
            }
            return UnityEngine.Rect.MinMaxRect(minX, minY, maxX, maxY);
        }

        private static UnityEngine.Vector2 Map(Projective2 value, UnityEngine.Vector2 point)
        {
            double divisor = value.M31 * point.x + value.M32 * point.y + value.M33;
            return new UnityEngine.Vector2(
                (float)((value.M11 * point.x + value.M12 * point.y + value.M13) / divisor),
                (float)((value.M21 * point.x + value.M22 * point.y + value.M23) / divisor)
            );
        }

        private static void AssertWorldPoint(
            GeometryObservationBatch batch,
            int id,
            Camera camera,
            Transform target,
            double displayWidth,
            double displayHeight,
            uint displayId
        )
        {
            var value = (GeometryValue.WorldPoint)
                ((GeometryObservationResult.Current)Value(batch, id).Result).Value;
            UnityEngine.Vector3 expected = camera.WorldToViewportPoint(target.position);
            Assert.That(
                value.Value.Point.X,
                Is.EqualTo((camera.rect.x + expected.x * camera.rect.width) * displayWidth)
                    .Within(0.1)
            );
            Assert.That(
                value.Value.Point.Y,
                Is.EqualTo(
                        displayHeight
                            - (camera.rect.y + expected.y * camera.rect.height) * displayHeight
                    )
                    .Within(0.1)
            );
            Assert.That(value.Value.Point.DisplayId, Is.EqualTo(new DisplayId(displayId)));
            Assert.That(value.Value.Depth, Is.EqualTo(expected.z).Within(0.001));
            Assert.That(value.Value.IsInsideViewport, Is.True);
        }

        private static GeometryObservation Observation(
            int value,
            GeometryObservationTarget target
        ) => new(ObservationId(value), target);

        private static GeometryObservationId ObservationId(int value) =>
            new(new Guid(value, 0, 0, new byte[8]));

        private static ObjectId Id(int value) => new(new Guid(value, 0, 0, new byte[8]));

        private static GeometryObservationValue Value(GeometryObservationBatch batch, int id) =>
            batch.Changed.Single(value => value.ObservationId.Equals(ObservationId(id)));

        private static void AssertUnavailable(
            GeometryObservationBatch batch,
            int id,
            GeometryUnavailable expected
        ) =>
            Assert.That(
                ((GeometryObservationResult.Unavailable)Value(batch, id).Result).Reason,
                Is.EqualTo(expected)
            );

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
            private readonly HashSet<ObjectId> uiObjects = new();

            public Camera? InputCamera { get; set; }

            public void Set(ObjectId id, GameObject value) => values[id] = value;

            public void SetUi(ObjectId id) => uiObjects.Add(id);

            public BattlementGeometryObjectKind LookupObject(
                ObjectId id,
                out GameObject? gameObject
            )
            {
                if (values.TryGetValue(id, out gameObject))
                    return BattlementGeometryObjectKind.World;
                return uiObjects.Contains(id)
                    ? BattlementGeometryObjectKind.Ui
                    : BattlementGeometryObjectKind.Missing;
            }
        }
    }
}
