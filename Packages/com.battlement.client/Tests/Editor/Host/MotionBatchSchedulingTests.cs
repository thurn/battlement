#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class MotionBatchSchedulingTests
    {
        [TestCase(false)]
        [TestCase(true)]
        public void VerifiedMotionCommandBlockingFlagControlsTheNextNativeBatchGroup(bool blocking)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            var session = new SessionId(Guid.NewGuid());
            ObjectId host = Id();
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, objects: new[] { Empty(host) })
            );
            harness.Runner.Connect();
            ObjectId clock = Id();
            ObjectId control = Id();
            ObjectId uiHost = Id();
            var ui = new VisualElement();
            ui.style.opacity = 0;
            BattlementMotionWorld motion = harness.Runner.UiDocumentsForTests.MotionWorldForTests;
            MotionDescriptor Definition(ObjectId id, MotionProperty property) =>
                SharedMotionDriverTests.Descriptor(id, clock, property, 0) with
                {
                    ControlId = control,
                    NamedTargets = new[]
                    {
                        new MotionNamedTarget(
                            "end",
                            SharedMotionDriverTests
                                .Descriptor(id, clock, property, 1)
                                .Slots[0]
                                .Target
                        ),
                    },
                };
            motion.Install(ui, uiHost, Definition(uiHost, MotionProperty.Opacity));
            motion
                .Prepare(
                    new BattlementWorldMotionTarget(Identity(host).transform),
                    host,
                    Definition(host, MotionProperty.LocalPositionX)
                )!
                .Commit();
            var start = new Command(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Motion.Control(
                    new MotionControlOperation(
                        control,
                        new MotionControlCommand.Start(
                            Id(),
                            1,
                            new MotionControlTarget.Variant("end")
                        )
                    )
                )
            );
            if (!blocking)
                start = start.Nonblocking();
            ObjectId marker = Id();
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    new ParallelCommandGroup<Command>(new[] { start }),
                    new ParallelCommandGroup<Command>(
                        new[]
                        {
                            new Command(
                                new CommandId(Guid.NewGuid()),
                                new CommandBody.Object.Create(Empty(marker))
                            ),
                        }
                    ),
                }
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
            Assert.That(HasIdentity(marker), Is.EqualTo(!blocking));
            if (!blocking)
            {
                DittoWorkObservation work = harness.Runner.ObserveDittoWork();
                Assert.That(work.ActiveFiniteTimelineCount, Is.Zero);
                Assert.That(work.HasHeldOperations, Is.True);
            }
            motion.SetControlledClock(clock, 500_000);
            motion.PostLayout();
            harness.Runner.RunFrame();
            Assert.That(Identity(host).transform.localPosition.x, Is.EqualTo(0.5).Within(0.00001));
            Assert.That(ui.style.opacity.value, Is.EqualTo(0.5).Within(0.00001));
            Assert.That(HasIdentity(marker), Is.EqualTo(!blocking));
            motion.SetControlledClock(clock, 1_000_000);
            motion.PostLayout();
            harness.Runner.RunFrame();
            Assert.That(HasIdentity(marker), Is.True);
            Assert.That(Identity(host).transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
            Assert.That(harness.Transport.BatchFailures, Is.Empty);
            Assert.That(harness.Clock.Elapsed, Is.EqualTo(TimeSpan.Zero));
        }

        private static ObjectId Id() => new(Guid.NewGuid());

        private static BattlementGameObject Empty(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static bool HasIdentity(ObjectId id) =>
            Object.FindObjectsByType<BattlementIdentity>().Any(identity => identity.Id == id.Value);

        private static BattlementIdentity Identity(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .Single(identity => identity.Id == id.Value);
    }
}
