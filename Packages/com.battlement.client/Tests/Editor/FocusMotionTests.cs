#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class FocusMotionTests
    {
        [Test]
        public void FocusHighlightRestoresStaticStyleAndTracksAuthoredChanges()
        {
            ObjectId host = new(Guid.NewGuid());
            ObjectId clock = new(Guid.NewGuid());
            var target = new VisualElement();
            target.style.opacity = 0.4f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            MotionDescriptor descriptor = Descriptor(host, clock);
            world.Install(target, host, descriptor);
            world.SetFocusVisible(host, true);
            Sample(world, clock);
            Assert.That(target.style.opacity.value, Is.EqualTo(0.9f).Within(0.001));
            world.SetFocusVisible(host, false);
            Sample(world, clock);
            Assert.That(target.style.opacity.value, Is.EqualTo(0.4f).Within(0.001));

            world.SetFocusVisible(host, true);
            Sample(world, clock);
            BattlementPreparedMotionAdmission prepared = world.Prepare(
                target,
                host,
                descriptor with
                {
                    Generation = 2,
                    Slots = new[] { descriptor.Slots[0] with { Generation = 2 } },
                }
            )!;
            target.style.opacity = 0.6f;
            prepared.Commit();
            world.SetFocusVisible(host, true);
            Sample(world, clock);
            world.SetFocusVisible(host, false);
            Sample(world, clock);
            Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.001));
        }

        [TestCase(false)]
        [TestCase(true)]
        public void FocusHighlightRevealsTheCurrentUnderlyingMotion(bool useGraph)
        {
            ObjectId host = new(Guid.NewGuid());
            ObjectId clock = new(Guid.NewGuid());
            ObjectId value = new(Guid.NewGuid());
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            MotionDescriptor descriptor = Descriptor(host, clock);
            if (useGraph)
                descriptor = descriptor with
                {
                    Values = new[]
                    {
                        new MotionValueDescriptor(
                            value,
                            new MotionValue.Scalar(0.3),
                            new MotionValueSource.Mutable()
                        ),
                    },
                    ValueBindings = new[] { new MotionValueBinding(MotionProperty.Opacity, value) },
                };
            else
                descriptor = descriptor with
                {
                    Slots = new[]
                    {
                        Slot(2, MotionLayer.Animate, 0.3) with
                        {
                            Target = Slot(2, MotionLayer.Animate, 0.3).Target with
                            {
                                TransitionEnd = new[]
                                {
                                    new MotionPropertyValue(
                                        MotionProperty.Opacity,
                                        new MotionValue.Scalar(0.5)
                                    ),
                                },
                            },
                        },
                        descriptor.Slots[0],
                    },
                };
            world.Install(target, host, descriptor);
            world.SetFocusVisible(host, true);
            Sample(world, clock);
            Assert.That(target.style.opacity.value, Is.EqualTo(0.9f).Within(0.001));
            world.SetFocusVisible(host, false);
            Sample(world, clock);
            Assert.That(
                target.style.opacity.value,
                Is.EqualTo(useGraph ? 0.3f : 0.5f).Within(0.001)
            );
        }

        private static void Sample(BattlementMotionWorld world, ObjectId clock)
        {
            world.AdvanceControlledClock(clock, 1_000);
            world.PreLayout();
            world.PostLayout();
        }

        private static MotionDescriptor Descriptor(ObjectId host, ObjectId clock) =>
            new(
                host,
                host,
                1,
                false,
                new[] { Slot(1, MotionLayer.FocusVisible, 0.9) },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );

        private static MotionSlotDescriptor Slot(ulong id, MotionLayer layer, double opacity) =>
            new(
                id,
                1,
                layer,
                new MotionTargetDescriptor(
                    new[]
                    {
                        new MotionPropertyTrack(
                            MotionProperty.Opacity,
                            new MotionValue[] { new MotionValue.Scalar(opacity) },
                            new TransitionDefinition(
                                new TransitionGenerator.Tween(
                                    1,
                                    new MotionEasing[] { new MotionEasing.Linear() },
                                    null
                                ),
                                0,
                                new MotionRepeat.None(),
                                0,
                                MotionRepeatType.Loop
                            )
                        ),
                    },
                    Array.Empty<MotionPropertyValue>()
                ),
                new MotionCallbackSubscriptions(false, false, false, false, false, false)
            );
    }
}
