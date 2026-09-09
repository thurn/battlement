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
            Assert.That(motion.PendingDiagnostic(), Does.Contain("finite-motion=1"));
            var journal = new List<MotionFrame>();

            for (var index = 0; index < 15; index++)
            {
                journal.Add(Advance(harness, motion, target));
            }
            motion.PreserveExactAdvanceState();

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
        public void ConfiguredFrameRateSurvivesRepeatedMotionInitialization()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var first = new DittoMotionController(harness.Runner);
            first.Begin(DittoMotion.Controlled);
            harness.Runner.SetDittoFrameRate(60);
            var second = new DittoMotionController(harness.Runner);

            second.Begin(DittoMotion.Controlled);
            TimeSpan before = harness.Runner.DittoElapsed;
            second.PrepareFrame(forceAdvance: true);

            Assert.That(
                harness.Runner.DittoElapsed - before,
                Is.EqualTo(TimeSpan.FromTicks(TimeSpan.TicksPerSecond / 60))
            );
        }

        [Test]
        public void InfiniteMotionFreezesAtTheFirstObservedStateAndDoesNotBlockSettlement()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Submit(
                harness,
                session,
                Tween(objectId, 30, repeat: new TweenRepeat.Forever(RepeatMode.Restart))
            );

            MotionFrame first = Advance(harness, motion, target);
            MotionFrame second = Advance(harness, motion, target);
            MotionFrame settled = Advance(harness, motion, target);

            Assert.That(first.Frame.HasInfiniteOperations, Is.True);
            Assert.That(first.Frame.HasPendingWork, Is.False);
            Assert.That(second.Frame.Elapsed, Is.EqualTo(first.Frame.Elapsed));
            Assert.That(settled.Frame.Elapsed, Is.EqualTo(first.Frame.Elapsed));
            Assert.That(settled.Position, Is.EqualTo(first.Position).Within(0.001f));
            Assert.That(settled.Frame.IsSettled, Is.True);
            Assert.That(motion.PendingDiagnostic(), Does.Contain("infinite-motion=1"));
        }

        [Test]
        public void PaintChangesResetSettlementWithoutLayoutOrStateChanges()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);

            DittoCommittedFrame initial = Advance(harness, motion, 10);
            DittoCommittedFrame textRepaint = Advance(harness, motion, 20);
            DittoCommittedFrame quiet = Advance(harness, motion, 20);
            DittoCommittedFrame settled = Advance(harness, motion, 20);

            Assert.That(initial.LayoutChanged, Is.False);
            Assert.That(textRepaint.LayoutChanged, Is.False);
            Assert.That(textRepaint.StateChanged, Is.False);
            Assert.That(textRepaint.PaintChanged, Is.True);
            Assert.That(textRepaint.IsSettled, Is.False);
            Assert.That(quiet.IsSettled, Is.False);
            Assert.That(settled.IsSettled, Is.True);
        }

        [Test]
        public void ContinuingUnownedPaintDeterministicallyRefusesSettlement()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);

            DittoCommittedFrame generatedContentMutation = null!;
            for (ulong fingerprint = 1; fingerprint <= 31; fingerprint++)
            {
                generatedContentMutation = Advance(harness, motion, fingerprint);
            }

            Assert.That(generatedContentMutation.PaintChanged, Is.True);
            Assert.That(generatedContentMutation.HasUncontrolledVisibleWork, Is.True);
        }

        [Test]
        public void RequestedControlledAdvanceOwnsPaintChanges()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);

            for (ulong fingerprint = 1; fingerprint <= 60; fingerprint++)
            {
                DittoCommittedFrame frame = Advance(
                    harness,
                    motion,
                    fingerprint,
                    forceAdvance: true
                );
                Assert.That(frame.HasUncontrolledVisibleWork, Is.False);
            }
        }

        [Test]
        public void FrozenPaintVerificationSettlesWithoutAdvancingFiniteWork()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, _) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Submit(harness, session, Tween(objectId, 30));
            DittoCommittedFrame advanced = Advance(harness, motion, 10, forceAdvance: true);
            _ = Advance(harness, motion, 10, preserveTime: true);
            DittoCommittedFrame quiet = Advance(harness, motion, 10, preserveTime: true);
            DittoCommittedFrame settled = Advance(harness, motion, 10, preserveTime: true);

            Assert.That(quiet.Elapsed, Is.EqualTo(advanced.Elapsed));
            Assert.That(settled.Elapsed, Is.EqualTo(advanced.Elapsed));
            Assert.That(settled.HasPendingWork, Is.True);
            Assert.That(settled.IsSettled, Is.True);
        }

        [Test]
        public void FrozenControlledStateRefusesContinuingPixelChanges()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, _) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Submit(harness, session, Tween(objectId, 30));
            _ = Advance(harness, motion, 1, forceAdvance: true);
            DittoCommittedFrame changed = null!;
            for (ulong fingerprint = 2; fingerprint <= 31; fingerprint++)
            {
                changed = Advance(harness, motion, fingerprint, preserveTime: true);
            }

            Assert.That(changed.HasPendingWork, Is.True);
            Assert.That(changed.HasUncontrolledVisibleWork, Is.True);
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

        private static DittoCommittedFrame Advance(
            BattlementTestHarness harness,
            DittoMotionController motion,
            ulong paintFingerprint,
            bool forceAdvance = false,
            bool preserveTime = false
        )
        {
            motion.PrepareFrame(forceAdvance, preserveTime);
            harness.Runner.RunFrame();
            harness.Runner.CompleteNativeFrame();
            return motion.ObserveCommittedFrame(paintFingerprint);
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

        private static Command Tween(
            ObjectId id,
            double x,
            double seconds = 1,
            TweenRepeat? repeat = null
        ) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.TweenLocalPosition(
                    id,
                    new Vector3(x, 0, 0),
                    new Tween(
                        TimeSpan.FromSeconds(seconds),
                        TimeSpan.Zero,
                        Easing.Linear,
                        repeat ?? new TweenRepeat.Once()
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
