#nullable enable
using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using MouseButton = UnityEngine.InputSystem.LowLevel.MouseButton;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementLogicalPointerTests : InputTestFixture
    {
        private Mouse mouse = null!;

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            mouse = InputSystem.AddDevice<Mouse>();
        }

        [TearDown]
        public override void TearDown()
        {
            base.TearDown();
        }

        [Test]
        public void GeometricWorldOrderAndCaptureSurviveReparentButNotHideOrDestroy()
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var first = new ObjectId(Guid.NewGuid());
            var second = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Region(first, 1), Region(second, 2));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 point = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            Move(harness, point, false);
            Move(harness, point, true);
            Assert.That(
                Events(harness).Last(e => e.Body is UiEventBody.PointerDown).TargetId,
                Is.EqualTo(second)
            );
            Assert.That(
                Events(harness).Count(e => e.Body is UiEventBody.PointerCapture),
                Is.EqualTo(1)
            );
            var parent = new GameObject("new parent");
            try
            {
                Identity(second).transform.SetParent(parent.transform, true);
                Move(harness, point + new UnityEngine.Vector2(180, 0), true);
                Assert.That(
                    Events(harness).Last(e => e.Body is UiEventBody.PointerMove).TargetId,
                    Is.EqualTo(second)
                );
                Assert.That(Losses(harness), Is.Zero);
                Identity(second).gameObject.SetActive(false);
                harness.Runner.RunFrame();
                Assert.That(Losses(harness), Is.EqualTo(1));
                Identity(second, true).gameObject.SetActive(true);
                Move(harness, point, true);
                Assert.That(
                    Events(harness).Count(e => e.Body is UiEventBody.PointerCapture),
                    Is.EqualTo(1)
                );
                Move(harness, point, false);
                Move(harness, point, true);
                Assert.That(
                    Events(harness).Count(e => e.Body is UiEventBody.PointerCapture),
                    Is.EqualTo(2)
                );
                Object.DestroyImmediate(Identity(second).gameObject);
                harness.Runner.RunFrame();
                Assert.That(Losses(harness), Is.EqualTo(2));
                Move(harness, point, false);
                Assert.That(Losses(harness), Is.EqualTo(2));
                Assert.That(Events(harness).Count(e => e.Body is UiEventBody.Click), Is.Zero);
            }
            finally
            {
                Object.DestroyImmediate(parent);
            }
        }

        [Test]
        public void LayerWinsOverDepthAndPreventedPressDoesNotCapture()
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var front = new ObjectId(Guid.NewGuid());
            var back = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Region(front, 2),
                Region(back, 1) with
                {
                    WorldPointer = new WorldPointerSettings(2, 1, true),
                    LocalTransform = new LocalTransform(
                        new Battlement.Vector3(0, 0, 2),
                        Quaternion.Identity,
                        Battlement.Vector3.One
                    ),
                }
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 point = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            Move(harness, point, false);
            harness.Transport.EnqueueUiEvent(
                new BattlementUiEventTransportResult(
                    BattlementTransportStatus.Success,
                    UiEventDisposition.PreventDefault,
                    BattlementFlatBufferResponseFixtures.Write(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                )
            );
            Move(harness, point, true);
            Assert.That(
                Events(harness).Last(e => e.Body is UiEventBody.PointerDown).TargetId,
                Is.EqualTo(back)
            );
            Assert.That(Events(harness).Any(e => e.Body is UiEventBody.PointerCapture), Is.False);
        }

        [Test]
        public void EnteringLogicalTargetWithHeldButtonDoesNotStartAnotherGesture()
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var card = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Region(card, 1));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 point = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            Move(harness, point + new UnityEngine.Vector2(10000, 10000), false);
            Move(harness, point + new UnityEngine.Vector2(10000, 10000), true);
            Move(harness, point, true);
            Move(harness, point, false);
            Assert.That(
                Events(harness)
                    .Any(e =>
                        e.Body
                            is UiEventBody.PointerDown
                                or UiEventBody.Click
                                or UiEventBody.PointerCapture
                    ),
                Is.False
            );
            Move(harness, point, true);
            Move(harness, point, false);
            Assert.That(Events(harness).Count(e => e.Body is UiEventBody.Click), Is.EqualTo(1));
        }

        [UnityTest]
        public IEnumerator OverlayGeometryBlocksWorldUntilExplicitPassthrough()
        {
            using var harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var card = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Region(card, 1));
            var rootId = new ObjectId(Guid.NewGuid());
            var documentId = new ObjectId(Guid.NewGuid());
            var overlayId = new ObjectId(Guid.NewGuid());
            GameObject owned = Battlement.UI.BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(
                    rootId,
                    new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize)
                )
            );
            try
            {
                harness.Runner.UiDocumentsForTests.Replace(
                    new[]
                    {
                        new UiDocument(
                            documentId,
                            rootId,
                            PickingMode: Prop<UiPickingMode>.Set(UiPickingMode.Ignore),
                            Children: new[]
                            {
                                new UiNode(
                                    overlayId,
                                    new UiElement.Box
                                    {
                                        Style = new UiStyle(
                                            Position: UiStyle.Set(UiPosition.Absolute),
                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(0)
                                            ),
                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Px(0)
                                            ),
                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(100)
                                            ),
                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                new UiLengthOrAuto.Percent(100)
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
                Camera camera = Identity(cameraId).GetComponent<Camera>();
                UnityEngine.Vector2 point = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
                Move(harness, point, false);
                Move(harness, point, true);
                Move(harness, point, false);
                Assert.That(
                    Events(harness)
                        .Any(e => e.TargetId == card && e.Body is UiEventBody.PointerDown),
                    Is.False
                );
                VisualElement overlay = owned
                    .GetComponent<UIDocument>()
                    .rootVisualElement.Children()
                    .Single();
                overlay.pickingMode = UnityEngine.UIElements.PickingMode.Ignore;
                yield return null;
                Move(harness, point, true);
                Move(harness, point, false);
                Assert.That(
                    Events(harness).Count(e => e.TargetId == card && e.Body is UiEventBody.Click),
                    Is.EqualTo(1)
                );
            }
            finally
            {
                Object.DestroyImmediate(owned);
            }
        }

        private static BattlementGameObject Region(ObjectId id, uint order) =>
            new(
                id,
                new GameObjectKind.BoxHitRegion(
                    new BoxHitRegionState(
                        new Battlement.Vector3(2, 2, 0.2),
                        Battlement.Vector3.Zero
                    )
                ),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Enum.GetValues(typeof(PointerEvent)).Cast<PointerEvent>().ToArray(),
                WorldPointer: new WorldPointerSettings(0, order, true)
            );

        private static void Connect(
            BattlementTestHarness harness,
            SessionId session,
            ObjectId cameraId,
            params BattlementGameObject[] objects
        )
        {
            var camera = new BattlementGameObject(
                cameraId,
                new GameObjectKind.Camera(
                    new CameraState() with
                    {
                        Projection = CameraProjection.Orthographic,
                        OrthographicSize = 3,
                    }
                ),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    new Battlement.Vector3(0, 0, -10),
                    Quaternion.Identity,
                    Battlement.Vector3.One
                ),
                Array.Empty<PointerEvent>()
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: objects.Append(camera).ToArray(),
                    inputCameraId: cameraId
                )
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            Physics.SyncTransforms();
        }

        private void Move(BattlementTestHarness harness, UnityEngine.Vector2 position, bool down)
        {
            InputSystem.QueueStateEvent(
                mouse,
                new MouseState { position = position }.WithButton(MouseButton.Left, down)
            );
            InputSystem.Update();
            harness.Runner.RunFrame();
        }

        private static BattlementIdentity Identity(ObjectId id, bool inactive = false) =>
            Object
                .FindObjectsByType<BattlementIdentity>(
                    inactive ? FindObjectsInactive.Include : FindObjectsInactive.Exclude
                )
                .Single(i => i.Id == id.Value);

        private static IEnumerable<UiEvent> Events(BattlementTestHarness harness) =>
            harness.Transport.UiEventActions.Select(a => a.Event);

        private static int Losses(BattlementTestHarness harness) =>
            Events(harness).Count(e => e.Body is UiEventBody.PointerCaptureOut);
    }
}
