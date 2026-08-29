#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.TestTools;
using UnityEngine.UIElements;
using ProtocolVector3 = Battlement.Vector3;

namespace Battlement.Tests
{
    public sealed class DittoInputTargetsTests
    {
        [UnityTest]
        public IEnumerator UiConditionsClipNestedTargetsAndReportTheBlockingUuid()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var lowerDocument = new ObjectId(Guid.NewGuid());
            var lowerRoot = new ObjectId(Guid.NewGuid());
            var clip = new ObjectId(Guid.NewGuid());
            var nested = new ObjectId(Guid.NewGuid());
            var upperDocument = new ObjectId(Guid.NewGuid());
            var upperRoot = new ObjectId(Guid.NewGuid());
            var blocker = new ObjectId(Guid.NewGuid());
            SessionId session = new(Guid.NewGuid());
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                objects: new[]
                {
                    Document(lowerDocument, lowerRoot, 0),
                    Document(upperDocument, upperRoot, 10),
                }
            ) with
            {
                Ui = new[]
                {
                    new UiDocument(
                        lowerDocument,
                        lowerRoot,
                        Children: new[]
                        {
                            new UiNode(
                                clip,
                                new UiElement.Box(),
                                new[] { new UiNode(nested, new UiElement.Button()) }
                            ),
                        }
                    ),
                    new UiDocument(
                        upperDocument,
                        upperRoot,
                        Children: new[] { new UiNode(blocker, new UiElement.Box()) }
                    ),
                },
            };
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(snapshot),
                        }
                    )
                )
            );
            harness.Runner.Connect();
            yield return null;
            yield return null;
            VisualElement clipElement = Element(harness, clip);
            VisualElement nestedElement = Element(harness, nested);
            clipElement.style.position = Position.Absolute;
            clipElement.style.left = 20;
            clipElement.style.top = 20;
            clipElement.style.width = 100;
            clipElement.style.height = 100;
            clipElement.style.overflow = Overflow.Hidden;
            nestedElement.style.position = Position.Absolute;
            nestedElement.style.left = 80;
            nestedElement.style.top = 80;
            nestedElement.style.width = 100;
            nestedElement.style.height = 100;
            VisualElement blockerElement = Element(harness, blocker);
            blockerElement.style.position = Position.Absolute;
            blockerElement.style.left = 0;
            blockerElement.style.top = 0;
            blockerElement.style.right = 0;
            blockerElement.style.bottom = 0;
            yield return null;

            var targets = new DittoInputTargets(
                harness.Runner,
                new Dictionary<string, ObjectId> { ["nested"] = nested },
                checked((uint)Screen.width),
                checked((uint)Screen.height)
            );
            var journal = new List<DittoInputResolution>();
            journal.Add(targets.Resolve(new DittoInputTarget.Object("nested")));
            Assert.That(journal[0].IsReachable, Is.False);
            Assert.That(journal[0].Candidates, Has.Count.EqualTo(25));
            Assert.That(
                journal[0].Candidates.Select(value => value.BlockingObject),
                Is.All.EqualTo(blocker)
            );
            float panelScale = nestedElement.panel.scaledPixelsPerPoint;
            Assert.That(journal[0].Bounds!.Value.width, Is.GreaterThan(0));
            Assert.That(
                journal[0].Bounds!.Value.width,
                Is.LessThan(nestedElement.worldBound.width * panelScale)
            );
            Assert.That(journal[0].Bounds!.Value.height, Is.GreaterThan(0));
            Assert.That(
                journal[0].Bounds!.Value.height,
                Is.LessThan(nestedElement.worldBound.height * panelScale)
            );

            Element(harness, upperRoot).style.display = DisplayStyle.None;
            yield return null;
            journal.Add(targets.Resolve(new DittoInputTarget.Object(nested.Value.ToString())));

            Assert.That(journal[1].IsReachable, Is.True);
            Assert.That(journal[1].Candidates, Has.Count.EqualTo(1));
            Assert.That(
                journal[1].Candidates[0].Position,
                Is.EqualTo(journal[1].Bounds!.Value.center)
            );
            Assert.That(
                targets
                    .Evaluate(new DittoObjectCondition("nested", DittoObjectState.Visible))
                    .Matches,
                Is.True
            );
            clipElement.style.opacity = 0;
            yield return null;
            Assert.That(
                targets
                    .Evaluate(new DittoObjectCondition("nested", DittoObjectState.Hidden))
                    .Matches,
                Is.True
            );
            Assert.That(
                targets
                    .Evaluate(
                        new DittoObjectCondition(Guid.NewGuid().ToString(), DittoObjectState.Absent)
                    )
                    .Matches,
                Is.True
            );
            nestedElement.SetEnabled(false);
            Assert.That(
                targets
                    .Evaluate(new DittoObjectCondition("nested", DittoObjectState.Disabled))
                    .Matches,
                Is.True
            );
        }

        [Test]
        public void WorldTargetUsesProductionRaycastAndCoordinatesUseTheRenderSurface()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var targetId = new ObjectId(Guid.NewGuid());
            var blockerId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: new[]
                    {
                        Cube(targetId, 0),
                        Cube(blockerId, -2),
                        CameraObject(cameraId),
                    },
                    inputCameraId: cameraId
                )
            );
            harness.Runner.Connect();
            Physics.SyncTransforms();
            var targets = new DittoInputTargets(
                harness.Runner,
                new Dictionary<string, ObjectId> { ["target"] = targetId },
                checked((uint)Screen.width),
                checked((uint)Screen.height)
            );

            DittoInputResolution blocked = targets.Resolve(new DittoInputTarget.Object("target"));

            Assert.That(blocked.IsReachable, Is.False);
            Assert.That(
                blocked.Candidates.Select(value => value.BlockingObject),
                Does.Contain(blockerId)
            );
            Assert.That(
                targets
                    .Evaluate(new DittoObjectCondition("target", DittoObjectState.Visible))
                    .Matches,
                Is.True
            );
            DittoConditionResult unsupported = targets.Evaluate(
                new DittoObjectCondition("target", DittoObjectState.Enabled)
            );
            Assert.That(unsupported.IsSupported, Is.False);
            Assert.That(unsupported.Diagnostic, Does.Contain(targetId.Value.ToString()));

            harness.Runner.TryGetObject(blockerId, out GameObject? blocker);
            blocker!.SetActive(false);
            Physics.SyncTransforms();
            DittoInputResolution reached = targets.Resolve(
                new DittoInputTarget.Object(targetId.Value.ToString())
            );
            Assert.That(reached.IsReachable, Is.True);
            Assert.That(reached.Candidates.Last().BlockingObject, Is.Null);

            DittoInputResolution coordinates = targets.Resolve(
                new DittoInputTarget.Coordinates(1, 1)
            );
            Assert.That(
                coordinates.Position,
                Is.EqualTo(new UnityEngine.Vector2(Screen.width - 1, Screen.height - 1))
            );
            harness.Runner.TryGetObject(targetId, out GameObject? target);
            target!.SetActive(false);
            Assert.That(
                targets
                    .Evaluate(new DittoObjectCondition("target", DittoObjectState.Hidden))
                    .Matches,
                Is.True
            );
        }

        private static BattlementGameObject Document(ObjectId id, ObjectId root, int order) =>
            new(
                id,
                new GameObjectKind.UiDocumentState(root, SortingOrder: order),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject CameraObject(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Camera(
                    new CameraState
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

        private static BattlementGameObject Cube(ObjectId id, double z) =>
            new(
                id,
                new GameObjectKind.Cube(),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    new ProtocolVector3(0, 0, z),
                    Quaternion.Identity,
                    ProtocolVector3.One
                ),
                Enum.GetValues(typeof(PointerEvent)).Cast<PointerEvent>().ToArray()
            );

        private static VisualElement Element(BattlementTestHarness harness, ObjectId id)
        {
            Assert.That(
                harness.Runner.UiDocumentsForTests.TryGet(id, out VisualElement? value),
                Is.True
            );
            return value!;
        }
    }
}
