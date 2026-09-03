#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class VariantMotionTests
    {
        [Test]
        public void StaggeredChildrenStartOnlyAtTheirForwardAndReverseBoundaries()
        {
            AssertOrder(
                StaggerDirection.Forward,
                new ulong[] { 320_000, 410_000, 500_000 },
                new[] { 0, 1, 2 }
            );
            AssertOrder(
                StaggerDirection.Reverse,
                new ulong[] { 500_000, 410_000, 320_000 },
                new[] { 2, 1, 0 }
            );
        }

        [Test]
        public void MidflightVariantReplacementCancelsTheOldGenerationWithoutAJmp()
        {
            ObjectId clock = Id("8597f7fd-889c-4589-98f8-6e1b0d0c9c7d");
            ObjectId host = Id("3bb62b38-4b3c-4fcb-adf8-b1bc8d83db94");
            var target = new VisualElement();
            target.style.opacity = 0;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                Descriptor(host, clock, 1, 1, 0, StaggerDirection.Forward, "east")
            );
            world.DrainEvents();
            world.SetControlledClock(clock, 400_000);
            world.PostLayout();
            float presentation = target.style.opacity.value;
            world.DrainEvents();

            world.Install(
                target,
                host,
                Descriptor(host, clock, 2, 0, 0, StaggerDirection.Reverse, "west")
            );
            MotionLifecycleEvent[] replacement = world.DrainEvents().ToArray();
            Assert.That(
                replacement.Any(value =>
                    value.Generation == 1 && value.Kind is MotionEventKind.Cancelled
                ),
                Is.True
            );
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(presentation).Within(0.00001));

            world.AdvanceControlledClock(clock, 100_000);
            world.PostLayout();
            Assert.That(
                target.style.opacity.value,
                Is.EqualTo(presentation * 0.9f).Within(0.00001)
            );
            world.AdvanceControlledClock(clock, 1_000_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0).Within(0.00001));
            Assert.That(world.DrainEvents().All(value => value.Generation == 2), Is.True);
        }

        private static void AssertOrder(
            StaggerDirection direction,
            ulong[] delays,
            int[] expectedOrder
        )
        {
            ObjectId clock = Id(
                direction == StaggerDirection.Forward
                    ? "d4d31972-af6d-47be-9e30-646cf4754713"
                    : "3d3bcde9-a24f-45ad-ae16-b273692a141d"
            );
            ObjectId[] hosts =
            {
                Id("27cc4aac-afd6-46ae-a133-f9713c8d7085"),
                Id("a90575a0-6c11-4bc4-9001-441ef97d7780"),
                Id("0377032f-e246-4c81-984c-1d421ea1a939"),
            };
            VisualElement[] targets = hosts.Select(_ => new VisualElement()).ToArray();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            for (int index = 0; index < hosts.Length; index++)
            {
                targets[index].style.opacity = 0;
                world.Install(
                    targets[index],
                    hosts[index],
                    Descriptor(hosts[index], clock, 1, 1, delays[index], direction, "east")
                );
            }
            world.DrainEvents();

            ulong[] boundaries = { 320_000, 410_000, 500_000 };
            for (int boundary = 0; boundary < boundaries.Length; boundary++)
            {
                world.SetControlledClock(clock, boundaries[boundary] - 1);
                world.PostLayout();
                Assert.That(world.DrainEvents(), Is.Empty);
                world.SetControlledClock(clock, boundaries[boundary] + 1);
                world.PostLayout();
                MotionLifecycleEvent started = world.DrainEvents().Single();
                Assert.That(started.Kind, Is.TypeOf<MotionEventKind.Started>());
                Assert.That(started.DescriptorId, Is.EqualTo(hosts[expectedOrder[boundary]]));
                Assert.That(started.ElapsedMicros, Is.EqualTo(boundaries[boundary]));
            }
        }

        private static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId clock,
            uint generation,
            double target,
            ulong delay,
            StaggerDirection direction,
            string route
        ) =>
            new(
                host,
                host,
                generation,
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
                                    Tween(delay)
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(true, false, false, true, false, true)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never,
                Variants: new MotionVariantResolution(
                    new[]
                    {
                        route,
                        "custom",
                        direction == StaggerDirection.Forward ? "forward" : "reverse",
                    },
                    true,
                    77,
                    0,
                    delay,
                    VariantWhen.BeforeChildren,
                    direction
                )
            );

        private static TransitionDefinition Tween(ulong delay) =>
            new(
                new TransitionGenerator.Tween(
                    1_000_000,
                    new MotionEasing[] { new MotionEasing.Linear() },
                    null
                ),
                checked((long)delay),
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
