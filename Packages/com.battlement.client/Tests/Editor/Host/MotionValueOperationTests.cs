#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class MotionValueOperationTests
    {
        [Test]
        public void SharedValuePlaybackPausesAndCompletesThroughTheCommandOperation()
        {
            var rendered = new GameObject("Shared value Motion");
            try
            {
                double now = 0;
                using var motion = new BattlementMotionWorld(
                    unscaledTime: () => now,
                    registerPlayerLoop: false
                );
                ObjectId value = Id();
                ObjectId uiHost = Id();
                ObjectId nativeHost = Id();
                var ui = new VisualElement();
                motion.Install(ui, uiHost, Descriptor(uiHost, value, MotionProperty.Opacity));
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        nativeHost,
                        Descriptor(nativeHost, value, MotionProperty.LocalPositionX)
                    )!
                    .Commit();
                ObjectId playback = Id();
                IBattlementCommandOperation operation = motion.ApplyValue(
                    value,
                    MotionValueOperationKind.Animate,
                    new MotionValue.Scalar(1),
                    playback,
                    1,
                    Linear(),
                    blocking: true
                )!;
                Assert.That(operation.IsComplete(TimeSpan.FromDays(1)), Is.False);
                now = 0.25;
                motion.PreLayout();
                Assert.That(ui.style.opacity.value, Is.EqualTo(0.25).Within(0.00001));
                Assert.That(
                    rendered.transform.localPosition.x,
                    Is.EqualTo(ui.style.opacity.value).Within(0.00001)
                );
                motion.ApplyValuePlayback(
                    playback,
                    1,
                    MotionPlaybackOperationKind.Pause,
                    0,
                    0,
                    MotionPlaybackDirection.Forward
                );
                Assert.That(((IBattlementHeldCommandOperation)operation).IsHeld, Is.True);
                now = 1.25;
                motion.PreLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(0.25).Within(0.00001));
                motion.ApplyValuePlayback(
                    playback,
                    1,
                    MotionPlaybackOperationKind.SetSpeed,
                    0,
                    2,
                    MotionPlaybackDirection.Forward
                );
                motion.ApplyValuePlayback(
                    playback,
                    1,
                    MotionPlaybackOperationKind.Play,
                    0,
                    0,
                    MotionPlaybackDirection.Forward
                );
                Assert.That(((IBattlementHeldCommandOperation)operation).IsHeld, Is.False);
                now = 1.625;
                motion.PreLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
                Assert.That(operation.IsComplete(TimeSpan.Zero), Is.True);
                Assert.That(
                    motion.DrainEventBatch()!.PlaybackEvents![0].Outcome,
                    Is.EqualTo(MotionPlaybackOutcome.Completed)
                );
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void ValueWriterFailureIsRetainedByItsCommandOperation()
        {
            var rendered = new GameObject("Failing value writer");
            ObjectId host = Id();
            ObjectId value = Id();
            using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
            try
            {
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        host,
                        Descriptor(host, value, MotionProperty.LocalPositionX)
                    )!
                    .Commit();
                IBattlementCommandOperation operation = motion.ApplyValue(
                    value,
                    MotionValueOperationKind.Animate,
                    new MotionValue.Scalar(1),
                    Id(),
                    1,
                    Linear()
                )!;
                UnityEngine.Object.DestroyImmediate(rendered);
                Assert.DoesNotThrow(() => motion.PreLayout());
                Assert.Throws<InvalidOperationException>(() => operation.IsComplete(TimeSpan.Zero));
                Assert.That(
                    motion.DrainEventBatch()!.PlaybackEvents![0].Outcome,
                    Is.EqualTo(MotionPlaybackOutcome.Failed)
                );
            }
            finally
            {
                if (rendered != null)
                    UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void ReducedWorldValueAppliesTheDestinationAndClearCancelsTheOperation()
        {
            var rendered = new GameObject("Reduced value Motion");
            try
            {
                double now = 0;
                ObjectId host = Id();
                ObjectId value = Id();
                using var motion = new BattlementMotionWorld(
                    unscaledTime: () => now,
                    registerPlayerLoop: false
                );
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        host,
                        Descriptor(host, value, MotionProperty.LocalPositionX) with
                        {
                            ReducedMotion = ReducedMotionPolicy.Always,
                        }
                    )!
                    .Commit();
                IBattlementCommandOperation operation = motion.ApplyValue(
                    value,
                    MotionValueOperationKind.Animate,
                    new MotionValue.Scalar(4),
                    Id(),
                    1,
                    Linear()
                )!;
                now = 0.25;
                motion.PreLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(4).Within(0.00001));
                motion.Clear();
                Assert.That(operation.IsComplete(TimeSpan.Zero), Is.True);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        [Test]
        public void PassiveSpringDoesNotChangeWhenSampledAgainAtTheSameTime()
        {
            var rendered = new GameObject("Passive spring Motion");
            try
            {
                double now = 0;
                using var motion = new BattlementMotionWorld(
                    unscaledTime: () => now,
                    registerPlayerLoop: false
                );
                ObjectId source = Id();
                ObjectId spring = Id();
                ObjectId uiHost = Id();
                ObjectId worldHost = Id();
                MotionDescriptor Definition(ObjectId host, MotionProperty property) =>
                    Descriptor(host, source, property) with
                    {
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
                        ValueBindings = new[] { new MotionValueBinding(property, spring) },
                    };
                var ui = new VisualElement();
                motion.Install(ui, uiHost, Definition(uiHost, MotionProperty.Opacity));
                motion
                    .Prepare(
                        new BattlementWorldMotionTarget(rendered.transform),
                        worldHost,
                        Definition(worldHost, MotionProperty.LocalPositionX)
                    )!
                    .Commit();
                motion.ApplyValue(
                    source,
                    MotionValueOperationKind.Set,
                    new MotionValue.Scalar(1),
                    default,
                    0,
                    null
                );
                motion.PreLayout();
                now = 0.1;
                motion.PreLayout();
                float first = rendered.transform.localPosition.x;
                Assert.That(first, Is.GreaterThan(0).And.LessThan(1));
                for (int index = 0; index < 3; index++)
                {
                    motion.PreLayout();
                    Assert.That(
                        rendered.transform.localPosition.x,
                        Is.EqualTo(first).Within(0.00001)
                    );
                    Assert.That(ui.style.opacity.value, Is.EqualTo(first).Within(0.00001));
                }
                now = 2;
                motion.PreLayout();
                Assert.That(rendered.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(rendered);
            }
        }

        private static ObjectId Id() => new(Guid.NewGuid());

        private static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId value,
            MotionProperty property
        ) =>
            new(
                host,
                host,
                1,
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Unscaled(),
                ReducedMotionPolicy.Never,
                Values: new[]
                {
                    new MotionValueDescriptor(
                        value,
                        new MotionValue.Scalar(0),
                        new MotionValueSource.Mutable()
                    ),
                },
                ValueBindings: new[] { new MotionValueBinding(property, value) }
            );

        private static TransitionDefinition Linear() =>
            new(
                new TransitionGenerator.Tween(
                    1_000_000,
                    new MotionEasing[] { new MotionEasing.Linear() }
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );
    }
}
