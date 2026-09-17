#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class MotionOwnershipTests
    {
        [TestCase(false, false)]
        [TestCase(true, false)]
        [TestCase(false, true)]
        [TestCase(true, true)]
        public void RetargetingOnePropertyPreservesDisjointImperativeTracks(
            bool world,
            bool transitionEnd
        )
        {
            var rendered = new GameObject("Disjoint Motion host");
            try
            {
                ObjectId host = new(Guid.NewGuid());
                ObjectId clock = new(Guid.NewGuid());
                ObjectId control = new(Guid.NewGuid());
                var ui = new VisualElement();
                ui.style.opacity = 0;
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                MotionProperty primary = world
                    ? MotionProperty.LocalPositionX
                    : MotionProperty.Opacity;
                MotionProperty other = world
                    ? MotionProperty.LocalPositionZ
                    : MotionProperty.ScaleX;
                MotionDescriptor Definition(MotionProperty property, double target) =>
                    SharedMotionDriverTests.Descriptor(host, clock, property, target);
                MotionDescriptor descriptor = Definition(primary, 0) with { ControlId = control };
                if (world)
                    motion
                        .Prepare(
                            new BattlementWorldMotionTarget(rendered.transform),
                            host,
                            descriptor
                        )!
                        .Commit();
                else
                    motion.Install(ui, host, descriptor);
                MotionTargetDescriptor first = Definition(primary, 1).Slots[0].Target;
                first = first with
                {
                    Tracks = transitionEnd
                        ? first.Tracks
                        : first
                            .Tracks.Concat(Definition(other, world ? 4 : 2).Slots[0].Target.Tracks)
                            .ToArray(),
                    TransitionEnd = transitionEnd
                        ? new[]
                        {
                            new MotionPropertyValue(other, new MotionValue.Scalar(world ? 4 : 2)),
                        }
                        : Array.Empty<MotionPropertyValue>(),
                };
                IBattlementCommandOperation previous = motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    new ObjectId(Guid.NewGuid()),
                    1,
                    new MotionControlTarget.Target(first)
                )!;
                motion.SetControlledClock(clock, 250_000);
                motion.PostLayout();
                motion.ApplyControl(
                    control,
                    MotionControlOperationKind.Start,
                    new ObjectId(Guid.NewGuid()),
                    1,
                    new MotionControlTarget.Target(Definition(primary, 0).Slots[0].Target)
                );
                Assert.That(previous.IsComplete(TimeSpan.Zero), Is.True);
                for (int index = 1; index <= 4; index++)
                {
                    motion.SetControlledClock(clock, (ulong)index * 250_000);
                    motion.PostLayout();
                    float value = world
                        ? rendered.transform.localPosition.z
                        : ui.style.scale.value.value.x;
                    float expected =
                        transitionEnd && index < 4
                            ? (world ? 0 : 1)
                            : (world ? index : 1 + index / 4f);
                    Assert.That(value, Is.EqualTo(expected).Within(0.00001));
                }
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }
    }
}
