#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.InputSystem.LowLevel;
using UnityEngine.TestTools;
using Object = UnityEngine.Object;
using ProtocolVector3 = Masonry.Vector3;

namespace Masonry.Tests
{
    public sealed class MasonryPointerInputTests : InputTestFixture
    {
        private Mouse? mouse;

        [SetUp]
        public override void Setup()
        {
            base.Setup();
            mouse = InputSystem.AddDevice<Mouse>("Masonry Test Mouse");
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var cameraObject = new GameObject("Authored Main Camera") { tag = "MainCamera" };
            Camera camera = cameraObject.AddComponent<Camera>();
            cameraObject.transform.position = new UnityEngine.Vector3(0, 0, -10);
            var session = new SessionId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            try
            {
                harness.Transport.EnqueueConnect(
                    FakeMasonryTransport.SnapshotResponse(
                        session,
                        objects: new[] { Cube(targetId, 0) },
                        useMainCamera: true
                    )
                );
                harness.Transport.DefaultSubmitResult = FakeMasonryTransport.ResponseResult(
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            Connect(harness, session, cameraId, Cube(targetId, 0));
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            MasonryIdentity targetIdentity = Identity(targetId);
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            MasonryGameObject target = Cube(targetId, 0, new[] { PointerEvent.Click });
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
        public void FocusLossCancelsDragAndRestoresPickupPosition()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
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
        public void SnapshotCancelsHeldPressWithoutUpOrClick()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            MasonryGameObject target = Cube(targetId, 0);
            Connect(harness, session, cameraId, target);
            Camera camera = Identity(cameraId).GetComponent<Camera>();
            UnityEngine.Vector2 position = camera.WorldToScreenPoint(
                Identity(targetId).transform.position
            );
            Move(harness, position, false);
            Move(harness, position, true);
            harness.Transport.EnqueuePoll(
                FakeMasonryTransport.SnapshotResponse(
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

        private static void Connect(
            MasonryTestHarness harness,
            SessionId session,
            ObjectId cameraId,
            params MasonryGameObject[] objects
        ) => ConnectCore(harness, session, cameraId, false, objects);

        private static void ConnectCore(
            MasonryTestHarness harness,
            SessionId session,
            ObjectId cameraId,
            bool inputDisabled,
            params MasonryGameObject[] objects
        )
        {
            MasonryGameObject camera = CameraObject(cameraId);
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    objects: objects.Append(camera).ToArray(),
                    inputCameraId: cameraId,
                    inputDisabled: inputDisabled
                )
            );
            harness.Transport.DefaultSubmitResult = FakeMasonryTransport.ResponseResult(
                new Response(session, Array.Empty<ResponseMessage<Command>>())
            );
            harness.Runner.Connect();
            Physics.SyncTransforms();
        }

        private static MasonryGameObject CameraObject(ObjectId cameraId) =>
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

        private static MasonryGameObject Cube(
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

        private void Move(MasonryTestHarness harness, UnityEngine.Vector2 position, bool leftButton)
        {
            InputSystem.QueueStateEvent(
                mouse!,
                new MouseState { position = position }.WithButton(MouseButton.Left, leftButton)
            );
            InputSystem.Update();
            harness.Runner.RunFrame();
        }

        private static MasonryIdentity Identity(ObjectId id) =>
            Object.FindObjectsByType<MasonryIdentity>().Single(identity => identity.Id == id.Value);

        private static Action[] Actions(MasonryTestHarness harness)
        {
            var actions = new List<Action>();
            foreach (byte[] bytes in harness.Transport.SubmitMessages)
            {
                try
                {
                    ClientMessage<CoreErrorCode, byte> message =
                        MasonryMessagePack.DeserializeClientMessage(
                            bytes,
                            new CoreErrorFormatter(),
                            new UnusedPayloadFormatter()
                        );
                    if (message is ClientMessage<CoreErrorCode, byte>.ActionMessage action)
                    {
                        actions.Add(action.Action);
                    }
                }
                catch (MessagePackSerializationException) { }
            }

            return actions.ToArray();
        }

        private sealed class CoreErrorFormatter : IMessagePackFormatter<CoreErrorCode>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                CoreErrorCode value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public CoreErrorCode Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }

        private sealed class UnusedPayloadFormatter : IMessagePackFormatter<byte>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                byte value,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }
    }
}
