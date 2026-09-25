#nullable enable

using System;
using System.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoPreparationClockTests
    {
        [TestCase(false, false, 1, false)]
        [TestCase(false, false, 60, false)]
        [TestCase(true, false, 60, false)]
        [TestCase(false, true, 1, false)]
        [TestCase(false, true, 60, false)]
        [TestCase(true, true, 60, false)]
        [TestCase(false, false, 60, true)]
        [TestCase(false, true, 60, true)]
        public void PreparationLatencyDoesNotAdvanceLogicalTime(
            bool instant,
            bool loadScene,
            int pendingFrames,
            bool forceAdvance
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            var motion = new DittoMotionController(harness.Runner);
            DittoMotion mode = instant ? DittoMotion.Instant : DittoMotion.Controlled;
            motion.Begin(mode);
            var objectId = new ObjectId(Guid.NewGuid());
            Submit(
                harness,
                session,
                new CommandBody.Object.Create(
                    new BattlementGameObject(
                        objectId,
                        new GameObjectKind.Empty(),
                        new ParentScene.Persistent(),
                        null,
                        true,
                        LocalTransform.Identity,
                        Array.Empty<PointerEvent>()
                    )
                )
            );
            Submit(
                harness,
                session,
                new CommandBody.Transform.TweenLocalPosition(
                    objectId,
                    new Vector3(30, 0, 0),
                    new Tween(
                        TimeSpan.FromSeconds(1),
                        TimeSpan.Zero,
                        Easing.Linear,
                        new TweenRepeat.Once()
                    )
                )
            );
            PreparedAsset addition = loadScene
                ? new PreparedAsset.Scene(new SceneAddress("app/clock-scene"))
                : new PreparedAsset.Texture(new TextureAddress("app/clock-texture"));
            if (!loadScene)
                harness.AssetStorage.EnqueuePending();
            Submit(
                harness,
                session,
                new CommandBody.Assets.ReplaceSet(
                    harness.AssetStorage.PrepareCalls.Append(addition).ToArray()
                )
            );
            System.Action complete;
            if (loadScene)
            {
                harness.AssetStorage.EnqueueSceneLoadPending();
                Submit(
                    harness,
                    session,
                    new CommandBody.Scene.Load(
                        new SceneId(Guid.NewGuid()),
                        ((PreparedAsset.Scene)addition).Address
                    )
                );
                for (int attempt = 0; attempt < 10; attempt++)
                {
                    if (harness.AssetStorage.SceneHandles.Any(handle => handle.Asset == addition))
                        break;
                    harness.Runner.RunFrame();
                }
                FakeSceneHandle scene = harness.AssetStorage.SceneHandles.Single(handle =>
                    handle.Asset == addition
                );
                Assert.That(scene.IsLoaded, Is.False);
                complete = scene.CompleteLoad;
            }
            else
            {
                FakeAssetHandle asset = harness.AssetStorage.Handles.Last();
                complete = () => asset.Complete();
            }

            for (int frame = 0; frame < pendingFrames; frame++)
            {
                harness.Clock.Advance(TimeSpan.FromSeconds(7));
                motion.PrepareFrame(forceAdvance);
                harness.Runner.RunFrame();
                harness.Runner.CompleteNativeFrame();
                DittoCommittedFrame observed = motion.ObserveCommittedFrame();
                Assert.That(observed.HasPendingWork, Is.True);
                Assert.That(observed.TimeAdvanced, Is.False);
            }
            Assert.That(harness.Runner.DittoElapsed, Is.EqualTo(TimeSpan.Zero));
            complete();
            harness.Runner.RunFrame();
            for (int frame = 0; frame < 32; frame++)
            {
                motion.PrepareFrame();
                harness.Runner.RunFrame();
                harness.Runner.CompleteNativeFrame();
                motion.ObserveCommittedFrame();
            }
            Assert.That(
                harness.Runner.DittoElapsed,
                Is.EqualTo(mode == DittoMotion.Controlled ? TimeSpan.FromSeconds(1) : TimeSpan.Zero)
            );
            Assert.That(harness.Runner.ObserveDittoWork().HasPendingWork, Is.False);
            Assert.That(
                harness.Runner.TryGetObject(objectId, out UnityEngine.GameObject? target),
                Is.True
            );
            Assert.That(target!.transform.localPosition.x, Is.EqualTo(30).Within(0.001));
        }

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            CommandBody body
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    new ParallelCommandGroup<Command>(
                        new[] { new Command(new CommandId(Guid.NewGuid()), body) }
                    ),
                },
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
        }
    }
}
