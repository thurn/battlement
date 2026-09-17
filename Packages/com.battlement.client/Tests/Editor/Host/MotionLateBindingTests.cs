#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class MotionLateBindingTests
    {
        [Test]
        public void BlockingStartRequiresAMountedTarget()
        {
            using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
            Assert.Throws<BattlementUiException>(() =>
                motion.ApplyControl(
                    Id(),
                    MotionControlOperationKind.Start,
                    Id(),
                    1,
                    new MotionControlTarget.Variant("end"),
                    blocking: true
                )
            );
            Assert.That(motion.DrainEventBatch()?.PlaybackEvents, Is.Null.Or.Empty);
        }

        [Test]
        public void NonblockingLateBindingUpdatesInfiniteReadiness()
        {
            using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
            ObjectId control = Id();
            var operation = motion.ApplyControl(
                control,
                MotionControlOperationKind.Start,
                Id(),
                1,
                new MotionControlTarget.Variant("end")
            )!;
            Assert.That(operation.IsInfinite, Is.False);
            Assert.That(((IBattlementHeldCommandOperation)operation).IsHeld, Is.True);
            MotionDescriptor descriptor = Descriptor(control, forever: true);
            motion.Install(new VisualElement(), descriptor.HostId, descriptor);
            Assert.That(operation.IsInfinite, Is.True);
            Assert.That(operation.IsComplete(TimeSpan.Zero), Is.False);
            operation.Cancel();
            Assert.That(operation.IsComplete(TimeSpan.Zero), Is.True);
        }

        [Test]
        public void BlockingControlRejectsALateInfiniteTargetBeforeAdmission()
        {
            using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
            ObjectId control = Id();
            MotionDescriptor first = Descriptor(control, forever: false);
            motion.Install(new VisualElement(), first.HostId, first);
            var operation = motion.ApplyControl(
                control,
                MotionControlOperationKind.Start,
                Id(),
                1,
                new MotionControlTarget.Variant("end"),
                blocking: true
            )!;
            MotionDescriptor second = Descriptor(control, forever: true);
            Assert.Throws<BattlementUiException>(() =>
                motion.Install(new VisualElement(), second.HostId, second)
            );
            Assert.That(motion.DescriptorCount, Is.EqualTo(1));
            motion.SetControlledClock(((MotionClockSource.Controlled)first.Clock).Value, 1_000_000);
            motion.PostLayout();
            Assert.That(operation.IsComplete(TimeSpan.Zero), Is.True);
        }

        private static MotionDescriptor Descriptor(ObjectId control, bool forever)
        {
            ObjectId host = Id();
            MotionDescriptor value = SharedMotionDriverTests.Descriptor(
                host,
                Id(),
                MotionProperty.Opacity,
                1
            );
            MotionPropertyTrack track = value.Slots[0].Target.Tracks[0];
            if (forever)
                track = track with
                {
                    Transition = track.Transition with { Repeat = new MotionRepeat.Forever() },
                };
            return value with
            {
                ControlId = control,
                Slots = Array.Empty<MotionSlotDescriptor>(),
                NamedTargets = new[]
                {
                    new MotionNamedTarget(
                        "end",
                        new MotionTargetDescriptor(
                            new[] { track },
                            Array.Empty<MotionPropertyValue>()
                        )
                    ),
                },
            };
        }

        private static ObjectId Id() => new(Guid.NewGuid());
    }
}
