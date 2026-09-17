#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;

namespace Battlement.Tests
{
    public sealed class MotionFailureOperationTests
    {
        [TestCase(false)]
        [TestCase(true)]
        public void FailureEndsSharedPlaybackOnceAndCancelsItsOtherTracks(bool destroyBoth)
        {
            var first = new GameObject("First Motion host");
            var second = new GameObject("Second Motion host");
            try
            {
                ObjectId firstId = new(Guid.NewGuid());
                ObjectId secondId = new(Guid.NewGuid());
                ObjectId clock = new(Guid.NewGuid());
                ObjectId control = new(Guid.NewGuid());
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                foreach (
                    (GameObject rendered, ObjectId host) in new[]
                    {
                        (first, firstId),
                        (second, secondId),
                    }
                )
                    motion
                        .Prepare(
                            new BattlementWorldMotionTarget(rendered.transform),
                            host,
                            SharedMotionDriverTests.Descriptor(
                                host,
                                clock,
                                MotionProperty.LocalPositionY,
                                4
                            ) with
                            {
                                ControlId = control,
                            }
                        )!
                        .Commit();
                IBattlementCommandOperation operation = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    new ObjectId(Guid.NewGuid()),
                    1,
                    new MotionControlTarget.Target(
                        SharedMotionDriverTests
                            .Descriptor(firstId, clock, MotionProperty.LocalPositionX, 4)
                            .Slots[0]
                            .Target
                    )
                )!;
                motion.SetControlledClock(clock, 250_000);
                motion.PostLayout();
                Assert.That(second.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
                UnityEngine.Object.DestroyImmediate(first);
                if (destroyBoth)
                    UnityEngine.Object.DestroyImmediate(second);
                motion.SetControlledClock(clock, 500_000);
                Assert.DoesNotThrow(() => motion.PostLayout());
                Assert.Throws<InvalidOperationException>(() => operation.IsComplete(TimeSpan.Zero));
                var events = motion.DrainEventBatch()!.PlaybackEvents!;
                Assert.That(events.Count, Is.EqualTo(1));
                Assert.That(events[0].Outcome, Is.EqualTo(MotionPlaybackOutcome.Failed));
                motion.SetControlledClock(clock, 750_000);
                Assert.DoesNotThrow(() => motion.PostLayout());
                Assert.That(motion.DrainEventBatch(), Is.Null);
                if (second != null)
                {
                    Assert.That(second.transform.localPosition.x, Is.Zero);
                    Assert.That(second.transform.localPosition.y, Is.EqualTo(3).Within(0.00001));
                }
            }
            finally
            {
                if (first != null)
                    UnityEngine.Object.DestroyImmediate(first);
                if (second != null)
                    UnityEngine.Object.DestroyImmediate(second);
            }
        }
    }
}
