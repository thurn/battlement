#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class PhysicalMotionTests
    {
        private static readonly TransitionDefinition Spring = new(
            new TransitionGenerator.Spring(
                new SpringConfiguration.Physical(100, 10, 1, null, 0.01, 0.005)
            ),
            0,
            new MotionRepeat.None(),
            0,
            MotionRepeatType.Loop
        );

        [Test]
        public void SpringRetargetKeepsSignedVelocityAcrossDroppedFrameJump()
        {
            ObjectId clock = Id("40bb2aae-3021-4f30-a435-07574a57da53");
            ObjectId host = Id("82df67bc-1643-4428-ac4a-15d8b7d03d86");
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, host, Descriptor(host, clock, 1, 1, Spring));

            world.SetControlledClock(clock, 200_000);
            world.PostLayout();
            MotionScalarSample interrupted = BattlementMotionScalarSampler.Sample(
                0,
                1,
                0,
                Spring,
                200_000
            );
            Assert.That(target.style.opacity.value, Is.EqualTo(interrupted.Value).Within(0.00001));
            Assert.That(interrupted.Velocity, Is.GreaterThan(0));

            world.Install(target, host, Descriptor(host, clock, 2, 0, Spring));
            world.AdvanceControlledClock(clock, 175_000);
            world.PostLayout();
            MotionScalarSample expected = BattlementMotionScalarSampler.Sample(
                interrupted.Value,
                0,
                interrupted.Velocity,
                Spring,
                175_000
            );
            Assert.That(target.style.opacity.value, Is.EqualTo(expected.Value).Within(0.00001));
            Assert.That(expected.Velocity, Is.LessThan(0));

            world.Install(target, host, Descriptor(host, clock, 3, 1, Spring));
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(expected.Value).Within(0.00001));
            world.AdvanceControlledClock(clock, 1_000);
            world.PostLayout();
            MotionScalarSample away = BattlementMotionScalarSampler.Sample(
                expected.Value,
                1,
                expected.Velocity,
                Spring,
                1_000
            );
            Assert.That(target.style.opacity.value, Is.EqualTo(away.Value).Within(0.00001));
            Assert.That(away.Velocity, Is.LessThan(0));
            world.AdvanceControlledClock(clock, 299_000);
            world.PostLayout();
            MotionScalarSample recovered = BattlementMotionScalarSampler.Sample(
                expected.Value,
                1,
                expected.Velocity,
                Spring,
                300_000
            );
            Assert.That(target.style.opacity.value, Is.EqualTo(recovered.Value).Within(0.00001));
            Assert.That(recovered.Velocity, Is.GreaterThan(0));
        }

        [Test]
        public void InertiaRespectsBoundsAndPauseResumeUsesLogicalTime()
        {
            ObjectId clock = Id("b4c56f05-2717-417c-995d-3c763fe83d28");
            ObjectId host = Id("f413478b-1ea2-482b-86f5-d58fd6250947");
            TransitionDefinition inertia = new(
                new TransitionGenerator.Inertia(
                    1,
                    0.8,
                    325_000,
                    0,
                    0.5,
                    0.001,
                    500,
                    24,
                    new InertiaTarget.Identity()
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, host, Descriptor(host, clock, 1, 1, inertia));
            world.SetControlledClock(clock, 120_000);
            world.PostLayout();
            float paused = target.style.opacity.value;
            world.Pause(host, 1, 1);
            world.AdvanceControlledClock(clock, 800_000);
            world.PostLayout();
            world.AdvanceControlledClock(clock, 800_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(paused));

            world.Play(host, 1, 1);
            world.AdvanceControlledClock(clock, 100_000);
            world.PostLayout();
            float caught = target.style.opacity.value;
            Assert.That(caught, Is.GreaterThan(paused));
            world.Install(target, host, Descriptor(host, clock, 2, 0.25, Tween()));
            world.AdvanceControlledClock(clock, 100_000);
            world.PostLayout();
            Assert.That(
                target.style.opacity.value,
                Is.EqualTo(caught + (0.25f - caught) * 0.1f).Within(0.00001)
            );
            world.AdvanceControlledClock(clock, 1_000_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.25f).Within(0.00001));
        }

        [Test]
        public void SpeedSeekAndReverseAreImmediateAndDeterministic()
        {
            ObjectId clock = Id("e4592800-91f3-4ca5-af8f-e25eda1d6b74");
            ObjectId host = Id("e30ac7a5-6a69-451f-a8dd-bfdcd4170866");
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, host, Descriptor(host, clock, 1, 1, Tween()));
            world.Seek(host, 1, 1, 750_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.75f).Within(0.00001));

            world.SetSpeed(host, 1, 1, 2);
            world.Play(host, 1, 1);
            world.AdvanceControlledClock(clock, 100_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.95f).Within(0.00001));

            world.Replay(host, 1, 1);
            world.SetDirection(host, 1, 1, MotionPlaybackDirection.Reverse);
            world.AdvanceControlledClock(clock, 250_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.SetSpeed(host, 1, 1, 0);
            world.AdvanceControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
        }

        [Test]
        public void TerminalCommandsEmitDistinctEventsAndCannotBeRevived()
        {
            AssertTerminal(
                "9c1b4bbb-ab27-4ad5-860d-6cebc6935f76",
                (world, id) => world.Stop(id, 1, 1),
                typeof(MotionEventKind.Stopped),
                0.25f
            );
            AssertTerminal(
                "82b6c731-f6b9-445f-8600-d796a7c43704",
                (world, id) => world.Cancel(id, 1, 1),
                typeof(MotionEventKind.Cancelled),
                0f
            );
            AssertTerminal(
                "e810dff4-cfb9-40a1-9dc8-7cbed0d08bb7",
                (world, id) => world.Complete(id, 1, 1),
                typeof(MotionEventKind.Completed),
                1f
            );
        }

        private static void AssertTerminal(
            string id,
            Action<BattlementMotionWorld, ObjectId> command,
            Type eventType,
            float expected
        )
        {
            ObjectId identity = Id(id);
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, identity, Descriptor(identity, identity, 1, 1, Tween()));
            world.DrainEvents();
            world.SetControlledClock(identity, 250_000);
            world.PostLayout();
            world.DrainEvents();
            command(world, identity);
            world.PostLayout();
            Assert.That(world.DrainEvents().Single().Kind.GetType(), Is.EqualTo(eventType));
            Assert.That(target.style.opacity.value, Is.EqualTo(expected).Within(0.00001));
            world.Play(identity, 1, 1);
            world.Seek(identity, 1, 1, 900_000);
            world.Complete(identity, 1, 1);
            world.AdvanceControlledClock(identity, 2_000_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(expected).Within(0.00001));
            Assert.That(world.DrainEvents(), Is.Empty);
        }

        private static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId clock,
            uint generation,
            double target,
            TransitionDefinition transition
        ) =>
            new(
                host,
                host,
                generation,
                Array.Empty<MotionPropertyValue>(),
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        1,
                        generation,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.Opacity,
                                    new MotionValue[] { new MotionValue.Scalar(target) },
                                    transition
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(true, true, true, true, true, true)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );

        private static TransitionDefinition Tween() =>
            new(
                new TransitionGenerator.Tween(
                    1_000_000,
                    new MotionEasing[] { new MotionEasing.Linear() },
                    null
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
