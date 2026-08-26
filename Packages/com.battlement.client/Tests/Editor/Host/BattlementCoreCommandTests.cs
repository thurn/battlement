#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;
using UiButton = Battlement.UiElement.Button;
using UiLabel = Battlement.UiElement.Label;
using UiToggle = Battlement.UiElement.Toggle;

namespace Battlement.Tests
{
    public sealed class BattlementCoreCommandTests
    {
        private static readonly PreparedAsset.Scene DefaultScene = new(
            new SceneAddress("battlement/tests/default-scene")
        );

        [Test]
        public void ObjectCommandsApplyInOrderAndRetainDestroyedIds()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            BattlementGameObject parent = Empty(parentId, positionX: 5);
            BattlementGameObject child = Empty(childId, parentId, positionX: 2);
            Submit(
                harness,
                session,
                Group(
                    Command(new CommandBody.Object.Create(parent)),
                    Command(new CommandBody.Object.Create(child)),
                    Command(new CommandBody.Object.SetActive(parentId, false)),
                    Command(new CommandBody.Object.Reparent(childId, null, true)),
                    Command(new CommandBody.Object.Destroy(parentId))
                )
            );

            Assert.That(Identity(parentId), Is.Null);
            BattlementIdentity retained = Identity(childId)!;
            Assert.That(retained.transform.parent!.name, Is.EqualTo("Battlement Persistent"));
            Assert.That(retained.transform.position.x, Is.EqualTo(7));

            Command reused = Command(new CommandBody.Object.Create(parent));
            Submit(harness, session, true, Group(reused));

            Assert.That(
                Failures(harness).Single().ErrorCode,
                Is.EqualTo(CoreErrorCode.DuplicateId)
            );
            Assert.That(Failures(harness).Single().CommandId, Is.EqualTo(reused.Id));
        }

        [Test]
        public void UiSnapshotReservesElementIdsAndDisposesItsRuntimePanel()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var labelId = new ObjectId(Guid.NewGuid());
            var buttonId = new ObjectId(Guid.NewGuid());
            BattlementGameObject documentObject = new(
                documentId,
                new GameObjectKind.UiDocumentState(rootId),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                objects: new[] { documentObject }
            ) with
            {
                Ui = new[]
                {
                    new UiDocument(
                        documentId,
                        rootId,
                        Children: new UiNode[]
                        {
                            new(labelId, new UiLabel { Text = "UI ONLINE" }),
                            new(buttonId, new UiButton { Text = "RUN" }),
                        }
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

            UIDocument nativeDocument = Object
                .FindObjectsByType<UIDocument>()
                .Single(value => value.gameObject.name == "Battlement UI Document");
            PanelSettings runtimePanel = nativeDocument.panelSettings;
            Assert.That(nativeDocument.rootVisualElement.Q<Label>().text, Is.EqualTo("UI ONLINE"));
            Assert.That(nativeDocument.rootVisualElement.Q<Button>().text, Is.EqualTo("RUN"));
            Command collision = Command(new CommandBody.Object.Create(Empty(rootId)));
            Submit(harness, session, true, Group(collision));
            Assert.That(
                Failures(harness).Single().ErrorCode,
                Is.EqualTo(CoreErrorCode.DuplicateId)
            );

            harness.Runner.Dispose();

            Assert.That(runtimePanel == null, Is.True);
            Assert.That(
                Object
                    .FindObjectsByType<UIDocument>()
                    .Any(value => value.gameObject.name == "Battlement UI Document"),
                Is.False
            );
        }

        [Test]
        public void DisabledGlobalInputSuppressesNativeBooleanProposals()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var documentId = new ObjectId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var toggleId = new ObjectId(Guid.NewGuid());
            BattlementGameObject documentObject = new(
                documentId,
                new GameObjectKind.UiDocumentState(rootId),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                objects: new[] { documentObject },
                inputDisabled: true
            ) with
            {
                Ui = new[]
                {
                    new UiDocument(
                        documentId,
                        rootId,
                        Children: new[]
                        {
                            new UiNode(
                                toggleId,
                                new UiToggle
                                {
                                    Name = "gated-toggle",
                                    Value = false,
                                    Events = new[] { UiEventKind.ValueCommitted },
                                }
                            ),
                        }
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
            Toggle toggle = Object
                .FindObjectsByType<UIDocument>()
                .Select(document => document.rootVisualElement.Q<Toggle>("gated-toggle"))
                .Single(value => value is not null)!;
            toggle.value = true;

            Assert.That(toggle.value, Is.False);
            Assert.That(harness.Transport.SubmitMessages, Is.Empty);
        }

        [Test]
        public void AssetReplacementRejectsLiveRemovalThenCommitsAfterRelease()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var retained = new PreparedAsset.Prefab(new PrefabAddress("game/retained"));
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    preparedAssets: new[] { retained }
                )
            );
            harness.Runner.Connect();
            IBattlementAssetLease lease = harness.Runner.AcquirePreparedAsset(retained);
            Command blocked = Command(
                new CommandBody.Assets.ReplaceSet(new PreparedAsset[] { DefaultScene })
            );

            Submit(harness, session, true, Group(blocked));

            Assert.That(Failures(harness).Single().ErrorCode, Is.EqualTo(CoreErrorCode.AssetInUse));
            Assert.That(harness.Runner.TryGetPreparedAsset(retained, out _), Is.True);

            lease.Dispose();
            var added = new PreparedAsset.Texture(new TextureAddress("game/added"));
            Command replacement = Command(
                new CommandBody.Assets.ReplaceSet(new PreparedAsset[] { DefaultScene, added })
            );
            Submit(harness, session, Group(replacement));

            Assert.That(harness.Runner.TryGetPreparedAsset(retained, out _), Is.False);
            Assert.That(harness.Runner.TryGetPreparedAsset(added, out _), Is.True);
        }

        [Test]
        public void SceneCommandsCutOverPrimaryDestroyDescendantsAndRejectPrimaryUnload()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var originalId = new SceneId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    preparedAssets: new PreparedAsset[] { DefaultScene },
                    scenes: new[] { new BattlementScene(originalId, DefaultScene.Address) },
                    primarySceneId: originalId
                )
            );
            harness.Runner.Connect();
            var second = new PreparedAsset.Scene(new SceneAddress("game/second-scene"));
            var secondId = new SceneId(Guid.NewGuid());
            var rootId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            Submit(
                harness,
                session,
                Group(
                    Command(
                        new CommandBody.Assets.ReplaceSet(
                            new PreparedAsset[] { DefaultScene, second }
                        )
                    )
                ),
                Group(Command(new CommandBody.Scene.Load(secondId, second.Address, true))),
                Group(
                    Command(
                        new CommandBody.Object.Create(
                            Empty(rootId, parentScene: new ParentScene.Specific(secondId))
                        )
                    ),
                    Command(
                        new CommandBody.Object.Create(
                            Empty(childId, rootId, parentScene: new ParentScene.Specific(secondId))
                        )
                    )
                )
            );

            Submit(harness, session, true, Group(Command(new CommandBody.Scene.Unload(secondId))));
            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.InvalidProperty)
            );
            Submit(
                harness,
                session,
                Group(Command(new CommandBody.Scene.SetPrimary(originalId))),
                Group(Command(new CommandBody.Scene.Unload(secondId)))
            );

            Assert.That(Identity(rootId), Is.Null);
            Assert.That(Identity(childId), Is.Null);

            Command reused = Command(new CommandBody.Scene.Load(secondId, second.Address));
            Submit(harness, session, true, Group(reused));
            Assert.That(Failures(harness).Last().ErrorCode, Is.EqualTo(CoreErrorCode.DuplicateId));
        }

        [Test]
        public void MaterialAndInputSelectionsExposeOnlySuccessfulImmediateEffects()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var firstAddress = new MaterialAddress("game/red");
            var secondAddress = new MaterialAddress("game/blue");
            Material first = NewMaterial(UnityEngine.Color.red);
            Material second = NewMaterial(UnityEngine.Color.blue);
            harness.AssetStorage.EnqueueValue(first);
            harness.AssetStorage.EnqueueValue(second);
            SessionId session = new(Guid.NewGuid());
            PreparedAsset[] assets =
            {
                new PreparedAsset.Material(firstAddress),
                new PreparedAsset.Material(secondAddress),
            };
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, preparedAssets: assets)
            );
            harness.Runner.Connect();
            var cubeId = new ObjectId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            Submit(
                harness,
                session,
                Group(
                    Command(
                        new CommandBody.Object.Create(
                            new BattlementGameObject(
                                cubeId,
                                new GameObjectKind.Cube(
                                    new[] { new MaterialAssignment(0, firstAddress) }
                                ),
                                new ParentScene.Persistent(),
                                null,
                                true,
                                LocalTransform.Identity,
                                Array.Empty<PointerEvent>()
                            )
                        )
                    ),
                    Command(
                        new CommandBody.Object.Create(
                            new BattlementGameObject(
                                cameraId,
                                new GameObjectKind.Camera(new CameraState()),
                                new ParentScene.Persistent(),
                                null,
                                true,
                                LocalTransform.Identity,
                                Array.Empty<PointerEvent>()
                            )
                        )
                    ),
                    Command(new CommandBody.Renderer.SetMaterial(cubeId, secondAddress)),
                    Command(
                        new CommandBody.Input.SetPointerEvents(
                            cubeId,
                            new[] { PointerEvent.Enter, PointerEvent.Click }
                        )
                    ),
                    Command(new CommandBody.Input.SetCamera(cameraId)),
                    Command(
                        new CommandBody.Input.SetGlobalKeys(
                            new[] { PhysicalKey.Escape, PhysicalKey.F1 }
                        )
                    )
                ),
                Group(
                    Command(
                        new CommandBody.Assets.ReplaceSet(
                            new PreparedAsset[]
                            {
                                DefaultScene,
                                new PreparedAsset.Material(secondAddress),
                            }
                        )
                    )
                )
            );

            BattlementIdentity cube = Identity(cubeId)!;
            Assert.That(cube.GetComponent<Renderer>().sharedMaterial, Is.SameAs(second));
            Assert.That(cube.GetComponent<Collider>(), Is.Not.Null);
            Assert.That(cube.IsPointerEventEnabled(PointerEvent.Enter), Is.True);
            Assert.That(cube.IsPointerEventEnabled(PointerEvent.Down), Is.False);
            Assert.That(harness.Runner.IsGlobalKeyEnabled(PhysicalKey.Escape), Is.True);
            Assert.That(harness.Runner.IsGlobalKeyEnabled(PhysicalKey.KeyA), Is.False);
            Assert.That(
                harness.Runner.TryGetPreparedAsset(new PreparedAsset.Material(firstAddress), out _),
                Is.False,
                "The old material lease must be released after assignment succeeds."
            );

            Command invalidSlot = Command(
                new CommandBody.Renderer.SetMaterial(cubeId, secondAddress, 5)
            );
            Submit(harness, session, true, Group(invalidSlot));

            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.InvalidProperty)
            );
            Assert.That(cube.GetComponent<Renderer>().sharedMaterial, Is.SameAs(second));
            Object.DestroyImmediate(first);
            Object.DestroyImmediate(second);
        }

        private static SessionId Connect(BattlementTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            return session;
        }

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params ParallelCommandGroup<Command>[] groups
        ) => Submit(harness, session, false, groups);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            bool reportsFailure,
            params ParallelCommandGroup<Command>[] groups
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                groups,
                Start: BatchStart.Now
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            if (reportsFailure)
            {
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });
        }

        private static ParallelCommandGroup<Command> Group(params Command[] commands) =>
            new(commands);

        private static Command Command(CommandBody body) =>
            new(new CommandId(Guid.NewGuid()), body);

        private static BattlementGameObject Empty(
            ObjectId id,
            ObjectId? parentId = null,
            double positionX = 0,
            ParentScene? parentScene = null
        ) =>
            new(
                id,
                new GameObjectKind.Empty(),
                parentScene ?? new ParentScene.Persistent(),
                parentId,
                true,
                new LocalTransform(
                    new Vector3(positionX, 0, 0),
                    Quaternion.Identity,
                    new Vector3(1, 1, 1)
                ),
                Array.Empty<PointerEvent>()
            );

        private static BattlementIdentity? Identity(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .SingleOrDefault(value => value.Id == id.Value);

        private static Material NewMaterial(UnityEngine.Color color)
        {
            var material = new Material(Shader.Find("Universal Render Pipeline/Lit"));
            material.color = color;
            return material;
        }

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Skip(1)
                .Where(bytes => bytes.Length > 1)
                .Select(bytes =>
                    BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes)
                )
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();
    }
}
