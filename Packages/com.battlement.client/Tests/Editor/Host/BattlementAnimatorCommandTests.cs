#nullable enable

using System;
using System.Linq;
using Newtonsoft.Json;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.Animations;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementAnimatorCommandTests
    {
        private static readonly int BlendParameter = Animator.StringToHash("Blend");
        private static readonly int IndexParameter = Animator.StringToHash("Index");
        private static readonly int VisibleParameter = Animator.StringToHash("Visible");

        [Test]
        public void PlayAndPersistentParametersMutateTheRootAnimatorWithoutInferredWaiting()
        {
            using AnimatorFixture fixture = AnimatorFixture.Create();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId) = Connect(harness, fixture);
            Command play = Command(
                new CommandBody.Animator.Play(objectId, "Running", NormalizedStartTime: 0.25)
            );
            Batch batch = Batch(
                session,
                Group(play),
                Group(
                    Command(new CommandBody.Animator.SetBool(objectId, "Visible", true)),
                    Command(new CommandBody.Animator.SetInt(objectId, "Index", 7)),
                    Command(new CommandBody.Animator.SetFloat(objectId, "Blend", 0.75)),
                    Command(new CommandBody.Animator.SetTrigger(objectId, "Pulse")),
                    Command(new CommandBody.Animator.SetSpeed(objectId, 0.5))
                )
            );

            Submit(harness, Response(session, batch));

            Animator animator = RootAnimator(objectId);
            Assert.That(animator.GetCurrentAnimatorStateInfo(0).IsName("Running"), Is.True);
            Assert.That(
                animator.GetCurrentAnimatorStateInfo(0).normalizedTime,
                Is.EqualTo(0.25f).Within(0.001f)
            );
            Assert.That(animator.GetBool(VisibleParameter), Is.True);
            Assert.That(animator.GetInteger(IndexParameter), Is.EqualTo(7));
            Assert.That(animator.GetFloat(BlendParameter), Is.EqualTo(0.75f).Within(0.001f));
            Assert.That(animator.speed, Is.EqualTo(0.5f));
            Assert.That(Failures(harness), Is.Empty);
        }

        [Test]
        public void PlayCompletionUsesOnlyTheExplicitWait()
        {
            using AnimatorFixture fixture = AnimatorFixture.Create();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId) = Connect(harness, fixture);
            Batch batch = Batch(
                session,
                Group(
                    Command(
                        new CommandBody.Animator.Play(
                            objectId,
                            "Running",
                            Wait: TimeSpan.FromMilliseconds(500)
                        )
                    )
                ),
                Group(Command(new CommandBody.Animator.SetBool(objectId, "Visible", true)))
            );

            Submit(harness, Response(session, batch));

            Animator animator = RootAnimator(objectId);
            Assert.That(animator.GetCurrentAnimatorStateInfo(0).IsName("Running"), Is.True);
            Assert.That(animator.GetBool(VisibleParameter), Is.False);
            harness.Clock.Advance(TimeSpan.FromMilliseconds(499));
            harness.Runner.RunFrame();
            Assert.That(animator.GetBool(VisibleParameter), Is.False);
            harness.Clock.Advance(TimeSpan.FromMilliseconds(1));
            harness.Runner.RunFrame();
            Assert.That(animator.GetBool(VisibleParameter), Is.True);
        }

        [Test]
        public void CrossFadeUsesItsOwnDurationAndAnIndependentExplicitWait()
        {
            using AnimatorFixture fixture = AnimatorFixture.Create();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId) = Connect(harness, fixture);
            Batch batch = Batch(
                session,
                Group(
                    Command(
                        new CommandBody.Animator.CrossFade(
                            objectId,
                            "Running",
                            TimeSpan.FromMilliseconds(200),
                            Wait: TimeSpan.FromMilliseconds(500)
                        )
                    )
                ),
                Group(Command(new CommandBody.Animator.SetInt(objectId, "Index", 9)))
            );

            Submit(harness, Response(session, batch));

            Animator animator = RootAnimator(objectId);
            Assert.That(animator.IsInTransition(0), Is.True);
            Assert.That(animator.GetNextAnimatorStateInfo(0).IsName("Running"), Is.True);
            animator.Update(0.19f);
            Assert.That(animator.IsInTransition(0), Is.True);
            animator.Update(0.02f);
            Assert.That(animator.GetCurrentAnimatorStateInfo(0).IsName("Running"), Is.True);
            Assert.That(animator.GetInteger(IndexParameter), Is.Zero);
            harness.Clock.Advance(TimeSpan.FromMilliseconds(500));
            harness.Runner.RunFrame();
            Assert.That(animator.GetInteger(IndexParameter), Is.EqualTo(9));
        }

        [Test]
        public void CancellingAnimatorWaitAdvancesTheNextGroup()
        {
            using AnimatorFixture fixture = AnimatorFixture.Create();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId) = Connect(harness, fixture);
            Command play = Command(
                new CommandBody.Animator.Play(objectId, "Running", Wait: TimeSpan.FromHours(1))
            );
            Batch batch = Batch(
                session,
                Group(play, Command(new CommandBody.Operation.Cancel(play.Id))),
                Group(Command(new CommandBody.Animator.SetBool(objectId, "Visible", true)))
            );

            Submit(harness, Response(session, batch));

            Assert.That(RootAnimator(objectId).GetBool(VisibleParameter), Is.True);
            Assert.That(harness.Clock.Elapsed, Is.EqualTo(TimeSpan.Zero));
            Assert.That(Failures(harness), Is.Empty);
        }

        [TestCase("missing-component", CoreErrorCode.ComponentMissing)]
        [TestCase("missing-state", CoreErrorCode.InvalidProperty)]
        [TestCase("missing-layer", CoreErrorCode.InvalidProperty)]
        [TestCase("invalid-start", CoreErrorCode.InvalidProperty)]
        [TestCase("invalid-cross-fade", CoreErrorCode.InvalidProperty)]
        [TestCase("wait-limit", CoreErrorCode.LimitExceeded)]
        [TestCase("cross-fade-limit", CoreErrorCode.LimitExceeded)]
        [TestCase("wrong-parameter-type", CoreErrorCode.InvalidProperty)]
        [TestCase("invalid-speed", CoreErrorCode.InvalidProperty)]
        public void InvalidAnimatorTargetsAndValuesFailTheBatch(
            string invalidCase,
            CoreErrorCode expected
        )
        {
            using AnimatorFixture fixture = AnimatorFixture.Create();
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId) = Connect(
                harness,
                fixture,
                invalidCase == "missing-component"
            );
            Command command = invalidCase switch
            {
                "missing-component" => Command(new CommandBody.Animator.Play(objectId, "Running")),
                "missing-state" => Command(new CommandBody.Animator.Play(objectId, "running")),
                "missing-layer" => Command(
                    new CommandBody.Animator.Play(objectId, "Running", uint.MaxValue)
                ),
                "invalid-start" => Command(
                    new CommandBody.Animator.Play(objectId, "Running", NormalizedStartTime: 1.1)
                ),
                "invalid-cross-fade" => Command(
                    new CommandBody.Animator.CrossFade(objectId, "Running", TimeSpan.Zero)
                ),
                "wait-limit" => Command(
                    new CommandBody.Animator.Play(
                        objectId,
                        "Running",
                        Wait: TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1)
                    )
                ),
                "cross-fade-limit" => Command(
                    new CommandBody.Animator.CrossFade(
                        objectId,
                        "Running",
                        TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1)
                    )
                ),
                "wrong-parameter-type" => Command(
                    new CommandBody.Animator.SetBool(objectId, "Index", true)
                ),
                "invalid-speed" => Command(
                    new CommandBody.Animator.SetSpeed(objectId, double.PositiveInfinity)
                ),
                _ => throw new ArgumentOutOfRangeException(nameof(invalidCase)),
            };

            SubmitExpectingFailure(harness, Response(session, Batch(session, Group(command))));

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.CommandId, Is.EqualTo(command.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(expected));
        }

        private static (SessionId Session, ObjectId ObjectId) Connect(
            BattlementTestHarness harness,
            AnimatorFixture fixture,
            bool withoutAnimator = false
        )
        {
            var session = new SessionId(Guid.NewGuid());
            var objectId = new ObjectId(Guid.NewGuid());
            if (withoutAnimator)
            {
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        session,
                        objects: new[] { Persistent(objectId, new GameObjectKind.Empty()) }
                    )
                );
            }
            else
            {
                harness.AssetStorage.EnqueueValue(fixture.Prefab);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        session,
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.Prefab(fixture.Address),
                        },
                        objects: new[]
                        {
                            Persistent(
                                objectId,
                                new GameObjectKind.Prefab(
                                    fixture.Address,
                                    Array.Empty<MaterialAssignment>(),
                                    Animator: new AnimatorState("Idle")
                                )
                            ),
                        }
                    )
                );
            }

            harness.Runner.Connect();
            Assert.That(
                harness.Transport.Calls.Last(),
                Is.Not.EqualTo("stop"),
                string.Join("\n", harness.Logger.Records.Select(record => record.Message))
            );
            return (session, objectId);
        }

        private static void Submit(BattlementTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void SubmitExpectingFailure(BattlementTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    new Response(response.SessionId, Array.Empty<ResponseMessage<Command>>())
                )
            );
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static Response Response(SessionId session, params Batch[] batches) =>
            new(
                session,
                batches
                    .Select(batch =>
                        (ResponseMessage<Command>)new ResponseMessage<Command>.BatchMessage(batch)
                    )
                    .ToArray()
            );

        private static Batch Batch(
            SessionId session,
            params ParallelCommandGroup<Command>[] groups
        ) => new(new BatchId(Guid.NewGuid()), session, groups);

        private static ParallelCommandGroup<Command> Group(params Command[] commands) =>
            new(commands);

        private static Command Command(CommandBody body) =>
            new(new CommandId(Guid.NewGuid()), body);

        private static BattlementGameObject Persistent(ObjectId id, GameObjectKind kind) =>
            new(
                id,
                kind,
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static Animator RootAnimator(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .Single(identity => identity.Id == id.Value)
                .GetComponent<Animator>();

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(TryDecode)
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();

        private static ClientMessage<CoreErrorCode, byte>? TryDecode(byte[] bytes)
        {
            try
            {
                return BattlementJson.DeserializeClientMessage<CoreErrorCode, byte>(bytes);
            }
            catch (JsonSerializationException)
            {
                return null;
            }
        }

        private sealed class AnimatorFixture : IDisposable
        {
            private readonly string controllerPath;
            private readonly RuntimeAnimatorController controller;
            private GameObject? prefab;

            private AnimatorFixture(
                string controllerPath,
                PrefabAddress address,
                RuntimeAnimatorController controller
            ) =>
                (this.controllerPath, Address, this.controller) = (
                    controllerPath,
                    address,
                    controller
                );

            public PrefabAddress Address { get; }

            public GameObject Prefab
            {
                get
                {
                    if (prefab != null)
                    {
                        return prefab;
                    }

                    prefab = new GameObject("Animator command fixture");
                    prefab.AddComponent<Animator>().runtimeAnimatorController = controller;
                    return prefab;
                }
            }

            public static AnimatorFixture Create()
            {
                string path = $"Assets/BattlementCommandAnimator-{Guid.NewGuid():N}.controller";
                AnimatorController controller = AnimatorController.CreateAnimatorControllerAtPath(
                    path
                );
                var clip = new AnimationClip { name = "Animator command pose" };
                clip.SetCurve(
                    string.Empty,
                    typeof(Transform),
                    "localPosition.x",
                    AnimationCurve.Constant(0, 1, 0)
                );
                AddState(controller, "Idle", clip);
                AddState(controller, "Running", clip);
                controller.AddParameter("Visible", AnimatorControllerParameterType.Bool);
                controller.AddParameter("Index", AnimatorControllerParameterType.Int);
                controller.AddParameter("Blend", AnimatorControllerParameterType.Float);
                controller.AddParameter("Pulse", AnimatorControllerParameterType.Trigger);
                AssetDatabase.AddObjectToAsset(clip, controller);
                AssetDatabase.SaveAssets();
                AssetDatabase.ImportAsset(path, ImportAssetOptions.ForceSynchronousImport);
                controller = AssetDatabase.LoadAssetAtPath<AnimatorController>(path);
                return new AnimatorFixture(
                    path,
                    new PrefabAddress("game/animator-command-fixture"),
                    controller
                );
            }

            public void Dispose()
            {
                if (prefab != null)
                {
                    Object.DestroyImmediate(prefab);
                }

                AssetDatabase.DeleteAsset(controllerPath);
            }

            private static void AddState(
                AnimatorController controller,
                string name,
                AnimationClip clip
            )
            {
                UnityEditor.Animations.AnimatorState state = controller
                    .layers[0]
                    .stateMachine.AddState(name);
                state.motion = clip;
            }
        }
    }
}
