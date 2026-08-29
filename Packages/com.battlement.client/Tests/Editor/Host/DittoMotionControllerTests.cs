#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class DittoMotionControllerTests
    {
        [Test]
        public void ControlledFramesPreserveIntermediateStateAndSettleAfterTwoQuietFrames()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Submit(harness, session, Tween(objectId, 30));
            var journal = new List<MotionFrame>();

            for (var index = 0; index < 15; index++)
            {
                journal.Add(Advance(harness, motion, target));
            }
            motion.PreserveExactWaitState();

            Assert.That(target.localPosition.x, Is.EqualTo(15f).Within(0.001f));
            Assert.That(journal[^1].Frame.Elapsed, Is.EqualTo(TimeSpan.FromMilliseconds(500)));

            DittoCommittedFrame settled;
            do
            {
                MotionFrame frame = Advance(harness, motion, target);
                journal.Add(frame);
                settled = frame.Frame;
            } while (!settled.IsSettled && journal.Count < 40);

            Assert.That(target.localPosition.x, Is.EqualTo(30f).Within(0.001f));
            Assert.That(settled.Index, Is.EqualTo(32));
            Assert.That(settled.QuietFrameCount, Is.EqualTo(2));
            Assert.That(settled.HasPendingWork, Is.False);
            Assert.That(settled.HasInfiniteOperations, Is.False);
            TestContext.Progress.WriteLine(string.Join(Environment.NewLine, journal));
        }

        [Test]
        public void ControlledRunRepeatsTheSameFrameValues()
        {
            float[] first = ControlledSamples();
            float[] second = ControlledSamples();

            Assert.That(first, Is.EqualTo(second));
            for (var index = 0; index < first.Length; index++)
            {
                Assert.That(first[index], Is.EqualTo(index + 1).Within(0.01f));
            }
        }

        [Test]
        public void InstantAndRealTimeUseTheirOwnedMotionRules()
        {
            using (
                BattlementTestHarness instant = BattlementTestHarness.Create(
                    useInstantAnimations: false
                )
            )
            {
                (SessionId session, ObjectId objectId, Transform target) = Connect(instant);
                var motion = new DittoMotionController(instant.Runner);
                motion.Begin(DittoMotion.Instant);

                Submit(instant, session, Tween(objectId, 30));

                Assert.That(target.localPosition.x, Is.EqualTo(30f).Within(0.001f));
                Assert.That(Advance(instant, motion, target).Frame.IsSettled, Is.False);
                Assert.That(Advance(instant, motion, target).Frame.IsSettled, Is.False);
                Assert.That(Advance(instant, motion, target).Frame.IsSettled, Is.True);
            }

            using BattlementTestHarness realTime = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId realSession, ObjectId realObject, Transform realTarget) = Connect(realTime);
            var realMotion = new DittoMotionController(realTime.Runner);
            realMotion.Begin(DittoMotion.RealTime);
            Submit(realTime, realSession, Tween(realObject, 30));

            realTime.Clock.Advance(TimeSpan.FromMilliseconds(500));
            Advance(realTime, realMotion, realTarget);

            Assert.That(realTarget.localPosition.x, Is.EqualTo(15f).Within(0.001f));
            Assert.That(
                DittoMotionController.UncontrolledWorkDiagnostic,
                Does.Contain("uncontrolled clock")
            );
        }

        private static float[] ControlledSamples()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Submit(harness, session, Tween(objectId, 5, 5.0 / 30.0));
            var samples = new float[5];
            for (var index = 0; index < samples.Length; index++)
            {
                samples[index] = Advance(harness, motion, target).Position;
            }
            return samples;
        }

        private static MotionFrame Advance(
            BattlementTestHarness harness,
            DittoMotionController motion,
            Transform target
        )
        {
            motion.PrepareFrame();
            harness.Runner.RunFrame();
            harness.Runner.CompleteNativeFrame();
            return new MotionFrame(motion.ObserveCommittedFrame(), target.localPosition.x);
        }

        private static (SessionId, ObjectId, Transform) Connect(BattlementTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            var objectId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: new[]
                    {
                        new BattlementGameObject(
                            objectId,
                            new GameObjectKind.Empty(),
                            new ParentScene.Persistent(),
                            null,
                            true,
                            LocalTransform.Identity,
                            Array.Empty<PointerEvent>()
                        ),
                    }
                )
            );
            harness.Runner.Connect();
            Transform target = Object
                .FindObjectsByType<BattlementIdentity>()
                .Single(value => value.Id == objectId.Value)
                .transform;
            return (session, objectId, target);
        }

        private static Command Tween(ObjectId id, double x, double seconds = 1) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.TweenLocalPosition(
                    id,
                    new Vector3(x, 0, 0),
                    new Tween(
                        TimeSpan.FromSeconds(seconds),
                        TimeSpan.Zero,
                        Easing.Linear,
                        new TweenRepeat.Once()
                    )
                )
            )
            {
                IsBlocking = false,
            };

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            Command command
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { new ParallelCommandGroup<Command>(new[] { command }) }
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

        private sealed record MotionFrame(DittoCommittedFrame Frame, float Position);
    }
}
