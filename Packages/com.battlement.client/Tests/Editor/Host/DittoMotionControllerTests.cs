#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.UI;
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
        public void ControlledBatchesDrainImmediateGroupsBeforeSamplingFiniteMotion()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = Connect(harness);
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(DittoMotion.Controlled);
            Command Position(int x) =>
                new(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.Transform.SetLocalPosition(objectId, new Vector3(x, 0, 0))
                );
            Submit(
                harness,
                session,
                Enumerable
                    .Range(1, 40)
                    .Select(Position)
                    .Append(Tween(objectId, 60) with { IsBlocking = true })
                    .Append(Position(100))
                    .ToArray()
            );
            Assert.That(target.localPosition.x, Is.EqualTo(40));
            Assert.That(harness.Runner.DittoElapsed, Is.EqualTo(TimeSpan.Zero));
            for (int frame = 0; frame < 15; frame++)
                Advance(harness, motion, target);
            Assert.That(target.localPosition.x, Is.EqualTo(50).Within(0.001));
            for (int frame = 0; frame < 15; frame++)
                Advance(harness, motion, target);
            Assert.That(target.localPosition.x, Is.EqualTo(100));
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
            Assert.That(motion.PendingDiagnostic(), Does.Contain("state-changed=False"));
            Assert.That(motion.PendingDiagnostic(), Does.Contain("layout-changed=False"));
            Assert.That(motion.PendingDiagnostic(), Does.Contain("paint-changed=True"));
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
        }

        [TestCase(true, false, 13)]
        [TestCase(true, true, 91)]
        [TestCase(false, false, 13)]
        [TestCase(false, true, 91)]
        public void DeterministicClockGraphsIgnoreWallTimeAndSettleAfterFiniteMotion(
            bool instant,
            bool scaled,
            int wallSeconds
        )
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            harness.Clock.Advance(TimeSpan.FromSeconds(wallSeconds));
            var controller = new DittoMotionController(harness.Runner);
            controller.Begin(instant ? DittoMotion.Instant : DittoMotion.Controlled);
            for (int index = 0; index < wallSeconds; index++)
                _ = Advance(harness, controller, 10);
            Assert.That(harness.Runner.DittoElapsed, Is.EqualTo(TimeSpan.Zero));
            (_, ObjectId host, Transform target) = Connect(harness);
            var value = new ObjectId(Guid.NewGuid());
            MotionClockSource source = scaled
                ? new MotionClockSource.Scaled()
                : new MotionClockSource.Unscaled();
            MotionDescriptor descriptor = SharedMotionDriverTests.Descriptor(
                host,
                host,
                MotionProperty.LocalPositionY,
                1
            ) with
            {
                Clock = source,
                Values = new[]
                {
                    new MotionValueDescriptor(
                        value,
                        new MotionValue.Scalar(0),
                        new MotionValueSource.Time(source)
                    ),
                },
                ValueBindings = new[]
                {
                    new MotionValueBinding(MotionProperty.LocalPositionX, value),
                },
            };
            harness
                .Runner.UiDocumentsForTests.MotionWorldForTests.Prepare(
                    new BattlementWorldMotionTarget(target),
                    host,
                    descriptor
                )!
                .Commit();

            MotionFrame frame = Advance(harness, controller, target);
            Assert.That(frame.Position, Is.EqualTo(1f / 30).Within(0.00001));
            Assert.That(frame.Frame.HasInfiniteOperations, Is.True);
            Assert.That(frame.Frame.HasPendingWork, Is.True);
            for (int index = 0; index < 40 && !frame.Frame.IsSettled; index++)
            {
                harness.Clock.Advance(TimeSpan.FromSeconds(wallSeconds));
                frame = Advance(harness, controller, target);
            }

            Assert.That(frame.Frame.IsSettled, Is.True);
            Assert.That(frame.Frame.HasPendingWork, Is.False);
            Assert.That(target.localPosition.y, Is.EqualTo(1).Within(0.00001));
            Assert.That(frame.Position, Is.EqualTo(1).Within(0.00001));
            harness.Clock.Advance(TimeSpan.FromDays(1));
            MotionFrame frozen = Advance(harness, controller, target);
            Assert.That(frozen.Frame.Elapsed, Is.EqualTo(frame.Frame.Elapsed));
            Assert.That(frozen.Position, Is.EqualTo(frame.Position));
        }

        [TestCase(true)]
        [TestCase(false)]
        public void PassiveSpringFinishesBeforeDeterministicCapture(bool instant)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var controller = new DittoMotionController(harness.Runner);
            controller.Begin(instant ? DittoMotion.Instant : DittoMotion.Controlled);
            (_, ObjectId host, Transform target) = Connect(harness);
            var source = new ObjectId(Guid.NewGuid());
            var spring = new ObjectId(Guid.NewGuid());
            MotionDescriptor descriptor = SharedMotionDriverTests.Descriptor(
                host,
                host,
                MotionProperty.LocalPositionX,
                1
            ) with
            {
                Slots = Array.Empty<MotionSlotDescriptor>(),
                Clock = new MotionClockSource.Unscaled(),
                Values = new[]
                {
                    new MotionValueDescriptor(
                        source,
                        new MotionValue.Scalar(0),
                        new MotionValueSource.Mutable()
                    ),
                    new MotionValueDescriptor(
                        spring,
                        new MotionValue.Scalar(0),
                        new MotionValueSource.Spring(
                            source,
                            new SpringConfiguration.Physical(100, 10, 1, null, null, null)
                        )
                    ),
                },
                ValueBindings = new[]
                {
                    new MotionValueBinding(MotionProperty.LocalPositionX, spring),
                },
            };
            BattlementMotionWorld world = harness.Runner.UiDocumentsForTests.MotionWorldForTests;
            world.Prepare(new BattlementWorldMotionTarget(target), host, descriptor)!.Commit();
            world.ApplyValue(
                source,
                MotionValueOperationKind.Set,
                new MotionValue.Scalar(1),
                default,
                0,
                null
            );
            MotionFrame frame = Advance(harness, controller, target);
            Assert.That(frame.Frame.HasPendingWork, Is.True);
            for (int index = 0; index < 300 && !frame.Frame.IsSettled; index++)
                frame = Advance(harness, controller, target);
            Assert.That(frame.Frame.IsSettled, Is.True);
            Assert.That(frame.Position, Is.EqualTo(1).Within(0.00001));
            Assert.That(frame.Frame.HasPendingWork, Is.False);
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
            BattlementMotionWorld world = harness.Runner.UiDocumentsForTests.MotionWorldForTests;
            world.PreLayout();
            world.PostLayout();
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
            params Command[] commands
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                commands
                    .Select(command => new ParallelCommandGroup<Command>(new[] { command }))
                    .ToArray()
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
