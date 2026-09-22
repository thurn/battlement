#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using MouseButton = UnityEngine.InputSystem.LowLevel.MouseButton;
using Object = UnityEngine.Object;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement.Tests
{
    public sealed class BattlementPointerInputTests : InputTestFixture
    {
        private Mouse? mouse;

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            mouse = InputSystem.AddDevice<Mouse>("Battlement Test Mouse");
        }

        [TearDown]
        public override void TearDown()
        {
            mouse = null;
            base.TearDown();
        }

        [Test]
        public void MainCameraSnapshotUsesTheTaggedSceneCameraForPointerInput()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var cameraObject = new GameObject("Authored Main Camera") { tag = "MainCamera" };
            Camera camera = cameraObject.AddComponent<Camera>();
            cameraObject.transform.position = new UnityEngine.Vector3(0, 0, -10);
            var session = new SessionId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            try
            {
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        session,
                        objects: new[] { Cube(targetId, 0) },
                        useMainCamera: true
                    )
                );
                harness.Transport.DefaultSubmitResult = () =>
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    );
                harness.Runner.Connect();
                Physics.SyncTransforms();
                UnityEngine.Vector2 position = camera.WorldToScreenPoint(
                    Identity(targetId).transform.position
                );

                Move(harness, position, false);

                var pointerEnter = (ActionBody.PointerEnter)Actions(harness).Single().Body;
                Assert.That(pointerEnter.ObjectId, Is.EqualTo(targetId));
            }
            finally
            {
                Object.DestroyImmediate(cameraObject);
            }
        }

        [Test]
        public void MouseHoverPressMoveAwayAndReturnEmitsOrderedPayloads()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var leftId = new ObjectId(Guid.NewGuid());
            var rightId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(leftId, -1), Cube(rightId, 1));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 left = camera.WorldToScreenPoint(
                Identity(leftId).transform.position
            );
            UnityEngine.Vector2 right = camera.WorldToScreenPoint(
                Identity(rightId).transform.position
            );

            Move(harness, left, false);
            Move(harness, left, true);
            Move(harness, right, true);
            Move(harness, left, true);
            Move(harness, left, false);

            Action[] actions = Actions(harness);
            Assert.That(
                actions.Select(action => action.Body.GetType().Name),
                Is.EqualTo(
                    new[]
                    {
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerDown),
                        nameof(ActionBody.PointerExit),
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerExit),
                        nameof(ActionBody.PointerEnter),
                        nameof(ActionBody.PointerUp),
                        nameof(ActionBody.PointerClick),
                    }
                )
            );
            Assert.That(actions.All(action => action.SessionId == session), Is.True);
            Assert.That(
                actions.Select(action => action.Id.Value).Distinct().Count(),
                Is.EqualTo(8)
            );

            var exit = (ActionBody.PointerExit)actions[2].Body;
            Assert.That(exit.ObjectId, Is.EqualTo(leftId));
            Assert.That(exit.ScreenPosition.X, Is.EqualTo(right.x).Within(0.01));
            Assert.That(exit.ScreenPosition.Y, Is.EqualTo(right.y).Within(0.01));
            Assert.That(exit.WorldHit.X, Is.EqualTo(-1).Within(0.01));
            Assert.That(exit.PointerId, Is.Zero);

            var click = (ActionBody.PointerClick)actions[^1].Body;
            Assert.That(click.ObjectId, Is.EqualTo(leftId));
            Assert.That(click.Button, Is.EqualTo(PointerButton.Left));
            Assert.That(click.WorldHit.Z, Is.EqualTo(-0.5).Within(0.01));
        }

        [Test]
        public void ClosestUnidentifiedColliderBlocksObjectsBehindIt()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(targetId, 0));
            GameObject blocker = GameObject.CreatePrimitive(PrimitiveType.Cube);
            blocker.transform.position = new UnityEngine.Vector3(0, 0, -3);
            try
            {
                Physics.SyncTransforms();
                Camera camera = Identity(cameraId).GetComponent<Camera>();
                Move(harness, camera.WorldToScreenPoint(UnityEngine.Vector3.zero), false);

                Assert.That(Actions(harness), Is.Empty);
            }
            finally
            {
                Object.DestroyImmediate(blocker);
            }
        }

        [Test]
        public void ReleaseOnDifferentRuntimeObjectEmitsUpWithoutClick()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var leftId = new ObjectId(Guid.NewGuid());
            var rightId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(leftId, -1), Cube(rightId, 1));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 left = camera.WorldToScreenPoint(
                Identity(leftId).transform.position
            );
            UnityEngine.Vector2 right = camera.WorldToScreenPoint(
                Identity(rightId).transform.position
            );

            Move(harness, left, false);
            Move(harness, left, true);
            Move(harness, right, true);
            Move(harness, right, false);

            Action[] actions = Actions(harness);
            Assert.That(actions.Any(action => action.Body is ActionBody.PointerClick), Is.False);
            var up = (ActionBody.PointerUp)actions.Last().Body;
            Assert.That(up.ObjectId, Is.EqualTo(rightId));
        }

        [Test]
        public void FocusLossDeactivationAndDestroyCancelPressedClicks()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(targetId, 0));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            BattlementIdentity targetIdentity = Identity(targetId);
            UnityEngine.Vector2 target = camera.WorldToScreenPoint(
                targetIdentity.transform.position
            );

            Move(harness, target, false);
            Move(harness, target, true);
            LogAssert.ignoreFailingMessages = true;
            try
            {
                harness.Runner.SendMessage("OnApplicationFocus", false);
                Move(harness, target, false);
                harness.Runner.SendMessage("OnApplicationFocus", true);
            }
            finally
            {
                LogAssert.ignoreFailingMessages = false;
            }

            Move(harness, target, true);
            targetIdentity.gameObject.SetActive(false);
            Move(harness, target, false);
            targetIdentity.gameObject.SetActive(true);
            Physics.SyncTransforms();
            Move(harness, target, true);
            Object.DestroyImmediate(targetIdentity.gameObject);
            Move(harness, target, false);

            Assert.That(
                Actions(harness).Count(action => action.Body is ActionBody.PointerClick),
                Is.Zero
            );
        }

        [Test]
        public void DisabledInputGateSuppressesAllPointerTransitions()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            ConnectCore(harness, session, cameraId, true, Cube(targetId, 0));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 target = camera.WorldToScreenPoint(
                Identity(targetId).transform.position
            );

            Move(harness, target, false);
            Move(harness, target, true);
            Move(harness, target, false);

            Assert.That(Actions(harness), Is.Empty);
        }

        [Test]
        public void TouchPointersAreProcessedInAscendingStableIdOrder()
        {
            InputSystem.RemoveDevice(mouse!);
            mouse = null;
            Touchscreen touchscreen = InputSystem.AddDevice<Touchscreen>();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var leftId = new ObjectId(Guid.NewGuid());
            var rightId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(leftId, -1), Cube(rightId, 1));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 left = camera.WorldToScreenPoint(
                Identity(leftId).transform.position
            );
            UnityEngine.Vector2 right = camera.WorldToScreenPoint(
                Identity(rightId).transform.position
            );

            BeginTouch(9, right, queueEventOnly: true, screen: touchscreen);
            BeginTouch(3, left, queueEventOnly: true, screen: touchscreen);
            InputSystem.Update();
            harness.Runner.RunFrame();

            Action[] actions = Actions(harness);
            Assert.That(
                actions.Select(action => PointerId(action.Body)),
                Is.EqualTo(new[] { 3, 3, 9, 9 })
            );
            Assert.That(actions[0].Body, Is.TypeOf<ActionBody.PointerEnter>());
            Assert.That(actions[1].Body, Is.TypeOf<ActionBody.PointerDown>());
        }

        [Test]
        public void DisabledEventKindsEmitNothingWhileClickOnlyStillTracksPress()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            BattlementGameObject target = Cube(targetId, 0, new[] { PointerEvent.Click });
            Connect(harness, session, cameraId, target);
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 position = camera.WorldToScreenPoint(
                Identity(targetId).transform.position
            );

            Move(harness, position, false);
            Move(harness, position, true);
            Move(harness, position, false);

            Assert.That(
                Actions(harness).Select(action => action.Body.GetType()),
                Is.EqualTo(new[] { typeof(ActionBody.PointerClick) })
            );
        }

        [TestCase(DragMode.SnapToPointer, 1.0)]
        [TestCase(DragMode.PreserveOffset, 0.75)]
        public void DraggableObjectFollowsPointerAndEmitsOnlyLifecycleActions(
            DragMode mode,
            double expectedX
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Cube(targetId, 0, Array.Empty<PointerEvent>(), mode)
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 pickup = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(0.25f, 0, 0)
            );
            UnityEngine.Vector2 destination = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(1, 0, 0)
            );

            Move(harness, pickup, false);
            Move(harness, pickup, true);
            Move(harness, destination, true);
            Move(harness, destination, false);

            Action[] actions = Actions(harness);
            Assert.That(
                actions.Select(action => action.Body.GetType()),
                Is.EqualTo(new[] { typeof(ActionBody.DragStart), typeof(ActionBody.DragEnd) })
            );
            var start = (ActionBody.DragStart)actions[0].Body;
            var end = (ActionBody.DragEnd)actions[1].Body;
            Assert.That(start.ObjectId, Is.EqualTo(targetId));
            Assert.That(start.WorldPosition.X, Is.Zero.Within(0.01));
            Assert.That(start.ScreenPosition.X, Is.EqualTo(pickup.x).Within(0.01));
            Assert.That(end.WorldPosition.X, Is.EqualTo(expectedX).Within(0.01));
            Assert.That(end.ScreenPosition.X, Is.EqualTo(destination.x).Within(0.01));
            Assert.That(
                Identity(targetId).transform.position.x,
                Is.EqualTo(expectedX).Within(0.01)
            );
        }

        [Test]
        public void NativeDragKeepsOwnershipAcrossALogicalWorldTarget()
        {
            // Chess captures cross an opposing piece with logical click handlers.
            // Raycast priority must not transfer an already captured native drag to
            // that piece: both input implementations must finish the original gesture.
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var dragged = new ObjectId(Guid.NewGuid());
            var crossed = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Cube(dragged, 0, Array.Empty<PointerEvent>(), DragMode.SnapToPointer),
                Cube(crossed, 1) with
                {
                    WorldPointer = new WorldPointerSettings(1, 0, false, true),
                }
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 pickup = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            UnityEngine.Vector2 destination = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(1, 0, 0)
            );
            Move(harness, pickup, false);
            Move(harness, pickup, true);
            Move(harness, destination, true);
            Move(harness, destination, false);
            ActionBody.DragEnd end = Actions(harness)
                .Select(a => a.Body)
                .OfType<ActionBody.DragEnd>()
                .Single();
            Assert.That(end.ObjectId, Is.EqualTo(dragged));
            Assert.That(end.WorldPosition.X, Is.EqualTo(1).Within(0.01));
            Assert.That(Identity(dragged).transform.position.x, Is.EqualTo(1).Within(0.01));
        }

        [Test]
        public void FocusLossCancelsDragAndRestoresPickupPosition()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Cube(targetId, 0, Array.Empty<PointerEvent>(), DragMode.SnapToPointer)
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 pickup = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            UnityEngine.Vector2 destination = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(1, 0, 0)
            );

            Move(harness, pickup, false);
            Move(harness, pickup, true);
            Move(harness, destination, true);
            LogAssert.ignoreFailingMessages = true;
            try
            {
                harness.Runner.SendMessage("OnApplicationFocus", false);
            }
            finally
            {
                LogAssert.ignoreFailingMessages = false;
            }

            Assert.That(Identity(targetId).transform.position.x, Is.Zero.Within(0.01));
            Assert.That(
                Actions(harness).Select(action => action.Body.GetType()),
                Is.EqualTo(new[] { typeof(ActionBody.DragStart) })
            );
        }

        [Test]
        public void AngledCameraDragKeepsPieceOnHorizontalBoardPlane()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Cube(targetId, 0, Array.Empty<PointerEvent>(), DragMode.SnapToPointer)
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            camera.transform.position = new UnityEngine.Vector3(0, 8, -4);
            camera.transform.rotation = UnityEngine.Quaternion.LookRotation(
                UnityEngine.Vector3.zero - camera.transform.position
            );
            UnityEngine.Vector3 destination = new(1, 0, 1);
            UnityEngine.Vector2 pickup = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            UnityEngine.Vector2 drop = camera.WorldToScreenPoint(destination);
            Physics.SyncTransforms();

            Move(harness, pickup, false);
            Move(harness, pickup, true);
            Move(harness, drop, true);
            Move(harness, drop, false);

            UnityEngine.Vector3 actual = Identity(targetId).transform.position;
            Assert.That(actual.x, Is.EqualTo(destination.x).Within(0.01));
            Assert.That(actual.y, Is.Zero.Within(0.01));
            Assert.That(actual.z, Is.EqualTo(destination.z).Within(0.01));
        }

        [Test]
        public void ControlledWorldHoverIgnoresFocusAndPhysicalMouseState()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(targetId, 0));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 target = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            BattlementControlledPointerLease lease = harness.Runner.BeginDittoInput("hover");
            try
            {
                InputSystem.QueueStateEvent(
                    mouse!,
                    new MouseState { position = UnityEngine.Vector2.zero }.WithButton(
                        MouseButton.Left,
                        true
                    )
                );
                InputSystem.Update();
                SetFocus(harness.Runner, false);
                harness.Runner.EnqueueDittoPointerSample(
                    0,
                    0,
                    target,
                    Array.Empty<PointerButton>(),
                    true,
                    false,
                    targetId,
                    1
                );

                harness.Runner.RunFrame();

                BattlementControlledPointerReceipt receipt = harness
                    .Runner.TakeDittoPointerReceipts()
                    .Single();
                Assert.That(receipt.Session, Is.EqualTo("hover"));
                Assert.That(receipt.Generation, Is.EqualTo(lease.Generation));
                Assert.That(receipt.Sequence, Is.Zero);
                Assert.That(receipt.ActualHit, Is.EqualTo(targetId));
                Assert.That(receipt.Route, Is.EqualTo("world-pointer"));
                Assert.That(
                    Actions(harness).Select(value => value.Body.GetType()),
                    Is.EqualTo(new[] { typeof(ActionBody.PointerEnter) })
                );
            }
            finally
            {
                harness.Runner.EndDittoInput();
            }
        }

        [Test]
        public void ControlledDragRetainsCaptureAcrossFocusAndUiCrossing()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(
                harness,
                session,
                cameraId,
                Cube(targetId, 0, Array.Empty<PointerEvent>(), DragMode.SnapToPointer)
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 pickup = camera.WorldToScreenPoint(UnityEngine.Vector3.zero);
            UnityEngine.Vector2 destination = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(1, 0, 0)
            );
            harness.Runner.BeginDittoInput("drag");
            try
            {
                harness.Runner.EnqueueDittoPointerSample(
                    0,
                    0,
                    pickup,
                    new[] { PointerButton.Left },
                    true,
                    false,
                    targetId,
                    1
                );
                harness.Runner.RunFrame();
                Assert.That(
                    harness.Runner.TakeDittoPointerReceipts().Single().CaptureOwner,
                    Is.EqualTo(targetId)
                );

                SetFocus(harness.Runner, false);
                InputSystem.QueueStateEvent(mouse!, new MouseState { position = pickup });
                InputSystem.Update();
                harness.Runner.EnqueueDittoPointerSample(
                    1,
                    0,
                    destination,
                    new[] { PointerButton.Left },
                    true,
                    false,
                    null,
                    2
                );
                harness.Runner.RunFrame();
                Assert.That(
                    harness.Runner.TakeDittoPointerReceipts().Single().CaptureOwner,
                    Is.EqualTo(targetId)
                );
                Assert.That(Identity(targetId).transform.position.x, Is.EqualTo(1).Within(0.01));

                harness.Runner.EnqueueDittoPointerSample(
                    2,
                    0,
                    destination,
                    Array.Empty<PointerButton>(),
                    true,
                    false,
                    null,
                    3
                );
                harness.Runner.RunFrame();
                Assert.That(harness.Runner.TakeDittoPointerReceipts(), Has.Count.EqualTo(1));
                Assert.That(
                    Actions(harness).Select(value => value.Body.GetType()),
                    Is.EqualTo(new[] { typeof(ActionBody.DragStart), typeof(ActionBody.DragEnd) })
                );
            }
            finally
            {
                harness.Runner.EndDittoInput();
            }
        }

        [UnityTest]
        public IEnumerator ControlledUiPressUsesNativeDispatchAndCapture()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var buttonId = new ObjectId(Guid.NewGuid());
            BattlementGameObject document = new(
                documentId,
                new GameObjectKind.UiDocumentState(
                    rootId,
                    new PanelSettingsValue(ScaleMode: PanelScaleMode.ConstantPixelSize)
                ),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(
                                FakeBattlementTransport.CompleteSnapshot(
                                    session,
                                    objects: new[] { document, CameraObject(cameraId) }
                                ) with
                                {
                                    InputCameraId = cameraId,
                                    Ui = new[]
                                    {
                                        new UiDocument(
                                            documentId,
                                            rootId,
                                            PickingMode: Prop<UiPickingMode>.Set(
                                                UiPickingMode.Ignore
                                            ),
                                            Children: new[]
                                            {
                                                new UiNode(
                                                    buttonId,
                                                    new UiElement.Button
                                                    {
                                                        Text = "controlled",
                                                        Events = new[]
                                                        {
                                                            UiEventKind.PointerDown,
                                                            UiEventKind.PointerCapture,
                                                            UiEventKind.PointerCaptureOut,
                                                            UiEventKind.Click,
                                                        },
                                                        Style = new UiStyle(
                                                            Position: UiStyle.Set(
                                                                UiPosition.Absolute
                                                            ),
                                                            Left: UiStyle.Set<UiLengthOrAuto>(
                                                                new UiLengthOrAuto.Px(40)
                                                            ),
                                                            Top: UiStyle.Set<UiLengthOrAuto>(
                                                                new UiLengthOrAuto.Px(40)
                                                            ),
                                                            Width: UiStyle.Set<UiLengthOrAuto>(
                                                                new UiLengthOrAuto.Px(180)
                                                            ),
                                                            Height: UiStyle.Set<UiLengthOrAuto>(
                                                                new UiLengthOrAuto.Px(80)
                                                            )
                                                        ),
                                                    }
                                                ),
                                            }
                                        ),
                                    },
                                }
                            ),
                        }
                    )
                )
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            yield return null;
            yield return null;
            Assert.That(
                harness.Runner.UiDocumentsForTests.TryGet(buttonId, out VisualElement? button),
                Is.True
            );
            UnityEngine.Vector2 position = new(
                button!.worldBound.center.x,
                Screen.height - button.worldBound.center.y
            );
            harness.Runner.BeginDittoInput("native-ui");
            try
            {
                SetFocus(harness.Runner, false);
                harness.Runner.EnqueueDittoPointerSample(
                    0,
                    0,
                    position,
                    new[] { PointerButton.Left },
                    true,
                    false,
                    buttonId,
                    1
                );
                harness.Runner.RunFrame();
                BattlementControlledPointerReceipt down = harness
                    .Runner.TakeDittoPointerReceipts()
                    .Single();
                Assert.That(down.ActualHit, Is.EqualTo(buttonId));
                Assert.That(down.CaptureOwner, Is.EqualTo(buttonId));
                Assert.That(down.Route, Is.EqualTo("ui-toolkit"));

                harness.Runner.EnqueueDittoPointerSample(
                    1,
                    0,
                    position,
                    Array.Empty<PointerButton>(),
                    true,
                    false,
                    buttonId,
                    2
                );
                harness.Runner.RunFrame();
                BattlementControlledPointerReceipt up = harness
                    .Runner.TakeDittoPointerReceipts()
                    .Single();
                Assert.That(up.ActualHit, Is.EqualTo(buttonId));
                Assert.That(up.CaptureOwner, Is.Null);
                Assert.That(
                    harness.Transport.UiEventActions.Select(value => value.Event.Body.GetType()),
                    Does.Contain(typeof(UiEventBody.Click))
                );
            }
            finally
            {
                harness.Runner.EndDittoInput();
            }
        }

        [Test]
        public void SnapshotCancelsHeldPressWithoutUpOrClick()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            BattlementGameObject target = Cube(targetId, 0);
            Connect(harness, session, cameraId, target);
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 position = camera.WorldToScreenPoint(
                Identity(targetId).transform.position
            );
            Move(harness, position, false);
            Move(harness, position, true);
            harness.Transport.EnqueuePoll(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: new[] { target, CameraObject(cameraId) },
                    inputCameraId: cameraId
                )
            );

            harness.Runner.RunFrame();
            Move(harness, position, false);

            ActionBody[] afterDown = Actions(harness)
                .Select(action => action.Body)
                .Skip(2)
                .ToArray();
            Assert.That(afterDown.Any(body => body is ActionBody.PointerUp), Is.False);
            Assert.That(afterDown.Any(body => body is ActionBody.PointerClick), Is.False);
        }

        [Test]
        public void IndependentBoxGeometryChangesTheActualPointerHitArea()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var hitId = new ObjectId(Guid.NewGuid());
            var faceId = new ObjectId(Guid.NewGuid());
            var initial = new BoxHitRegionState(ProtocolVector3.One, ProtocolVector3.Zero);
            BattlementGameObject hit = Cube(hitId, 0) with
            {
                Kind = new GameObjectKind.BoxHitRegion(initial),
            };
            BattlementGameObject face = Cube(faceId, 0, Array.Empty<PointerEvent>());
            Connect(harness, session, cameraId, hit, face);
            Assert.That(
                harness.Runner.IsInputAvailable,
                Is.True,
                string.Join("\n", harness.Logger.Records.Select(value => value.Message))
            );
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            GameObject native = Identity(hitId).gameObject;
            Assert.That(native.GetComponent<Renderer>(), Is.Null);
            UnityEngine.Vector2 outside = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(1.5f, 0, 0)
            );
            Move(harness, outside, true);
            Move(harness, outside, false);
            Assert.That(
                Actions(harness).OfType<Action>().Any(a => a.Body is ActionBody.PointerClick),
                Is.False
            );
            var resized = new BoxHitRegionState(
                new ProtocolVector3(2, 1, 1),
                new ProtocolVector3(1, 0, 0)
            );
            var commands = new[]
            {
                new Command(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.Transform.SetLocalScale(faceId, new ProtocolVector3(2, 1, 1))
                ),
                new Command(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.SetBoxHitRegion(hitId, resized)
                ),
            };
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { new ParallelCommandGroup<Command>(commands) },
                Start: BatchStart.Now
            );
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.BatchMessage(batch),
                        }
                    )
                )
            );
            harness.Runner.Submit(new byte[] { 1 });
            Physics.SyncTransforms();
            Assert.That(Identity(hitId).gameObject, Is.SameAs(native));
            Assert.That(native.GetComponent<BoxCollider>().center.x, Is.EqualTo(1));
            Assert.That(Identity(faceId).transform.localScale.x, Is.EqualTo(2));
            Move(harness, outside, true);
            Move(harness, outside, false);
            ActionBody.PointerClick click = Actions(harness)
                .Select(a => a.Body)
                .OfType<ActionBody.PointerClick>()
                .Single();
            Assert.That(click.ObjectId, Is.EqualTo(hitId));
            Assert.That(click.WorldHit.X, Is.EqualTo(1.5).Within(0.01));
            UnityEngine.Vector2 excluded = camera.WorldToScreenPoint(
                new UnityEngine.Vector3(-0.25f, 0, 0)
            );
            Move(harness, excluded, true);
            Move(harness, excluded, false);
            Assert.That(
                Actions(harness).Select(a => a.Body).OfType<ActionBody.PointerClick>().Count(),
                Is.EqualTo(1)
            );
        }

        private static void Connect(
            BattlementTestHarness harness,
            SessionId session,
            ObjectId cameraId,
            params BattlementGameObject[] objects
        ) => ConnectCore(harness, session, cameraId, false, objects);

        private static void ConnectCore(
            BattlementTestHarness harness,
            SessionId session,
            ObjectId cameraId,
            bool inputDisabled,
            params BattlementGameObject[] objects
        )
        {
            BattlementGameObject camera = CameraObject(cameraId);
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: objects.Append(camera).ToArray(),
                    inputCameraId: cameraId,
                    inputDisabled: inputDisabled
                )
            );
            harness.Transport.DefaultSubmitResult = () =>
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                );
            harness.Runner.Connect();
            Physics.SyncTransforms();
        }

        private static BattlementGameObject CameraObject(ObjectId cameraId) =>
            new(
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
                    new ProtocolVector3(0, 0, -10),
                    Quaternion.Identity,
                    ProtocolVector3.One
                ),
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject Cube(
            ObjectId id,
            double x,
            IReadOnlyList<PointerEvent>? events = null,
            DragMode? dragMode = null
        ) =>
            new(
                id,
                new GameObjectKind.Cube(),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    new ProtocolVector3(x, 0, 0),
                    Quaternion.Identity,
                    ProtocolVector3.One
                ),
                events ?? Enum.GetValues(typeof(PointerEvent)).Cast<PointerEvent>().ToArray(),
                dragMode
            );

        private static int PointerId(ActionBody body) =>
            body switch
            {
                ActionBody.PointerEnter value => value.PointerId,
                ActionBody.PointerDown value => value.PointerId,
                _ => throw new ArgumentException("Expected a pointer action.", nameof(body)),
            };

        private void Move(
            BattlementTestHarness harness,
            UnityEngine.Vector2 position,
            bool leftButton
        )
        {
            InputSystem.QueueStateEvent(
                mouse!,
                new MouseState { position = position }.WithButton(MouseButton.Left, leftButton)
            );
            InputSystem.Update();
            harness.Runner.RunFrame();
        }

        private static BattlementIdentity Identity(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .Single(identity => identity.Id == id.Value);

        private static void SetFocus(BattlementRunner runner, bool focused) =>
            typeof(BattlementRunner)
                .GetMethod("OnApplicationFocus", BindingFlags.Instance | BindingFlags.NonPublic)!
                .Invoke(runner, new object[] { focused });

        private static Action[] Actions(BattlementTestHarness harness) =>
            harness
                .Transport.Actions.Where(action =>
                    action.Body is not ActionBody.ApplicationStateChanged
                )
                .ToArray();
    }
}
