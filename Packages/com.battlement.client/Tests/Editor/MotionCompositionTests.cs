#nullable enable

using System;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.Tests
{
    public sealed class MotionCompositionTests
    {
        [Test]
        public void SharedAudioPhaseComposesWithLocalPressAndLateMounts()
        {
            ObjectId time = Id(),
                factor = Id(),
                range = Id(),
                audio = Id(),
                clock = Id();
            ulong playhead = 500_000;
            bool reduced = false;
            var values = new[]
            {
                new MotionValueDescriptor(
                    time,
                    new MotionValue.Scalar(0),
                    new MotionValueSource.Time(new MotionClockSource.Audio(audio))
                ),
                new MotionValueDescriptor(
                    range,
                    new MotionValue.Scalar(1),
                    new MotionValueSource.Range(
                        time,
                        new MotionValue[] { new MotionValue.Scalar(0), new MotionValue.Scalar(1) },
                        new MotionValue[]
                        {
                            new MotionValue.Scalar(1),
                            new MotionValue.Scalar(1.02),
                        },
                        true
                    )
                ),
                new MotionValueDescriptor(
                    factor,
                    new MotionValue.Scalar(1),
                    new MotionValueSource.Expression(
                        new MotionExpressionOperation.Power(1),
                        new[] { range }
                    )
                ),
            };
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                reducedMotion: () => reduced,
                audioTime: _ => new MotionClockSample(playhead, false)
            );
            var first = new VisualElement();
            first.style.scale = new Scale(Vector2.one);
            ObjectId host = Id();
            var descriptor = Descriptor(host, clock, factor, values, 0.955);
            world.Install(first, host, RoundTrip(descriptor));
            world.PreLayout();
            world.PostLayout();
            world.AdvanceControlledClock(clock, 500_000);
            world.PreLayout();
            world.PostLayout();
            AssertScale(first, 0.9775 * 1.01);

            var late = new VisualElement();
            late.style.scale = new Scale(Vector2.one);
            ObjectId lateHost = Id();
            world.Install(
                late,
                lateHost,
                RoundTrip(Descriptor(lateHost, clock, factor, values, 1))
            );
            world.PreLayout();
            world.PostLayout();
            AssertScale(late, 1.01);
            Assert.That(world.GraphNodeCount, Is.EqualTo(3), "Controls share one graph.");
            Assert.That(
                world.DrainEventBatch()?.ValueSamples ?? Array.Empty<MotionValueSample>(),
                Is.Empty,
                "Audio presentation does not send frame samples to Rust."
            );

            playhead = 750_000;
            world.PreLayout();
            world.PostLayout();
            AssertScale(first, 0.9775 * 1.015);
            AssertScale(late, 1.015);

            reduced = true;
            world.PreLayout();
            world.PostLayout();
            AssertScale(first, 0.955);
            AssertScale(late, 1);

            var changedValues = (MotionValueDescriptor[])values.Clone();
            var changedRange = (MotionValueSource.Range)changedValues[1].Source;
            changedValues[1] = changedValues[1] with
            {
                Source = changedRange with
                {
                    Output = new MotionValue[]
                    {
                        new MotionValue.Scalar(1),
                        new MotionValue.Scalar(2),
                    },
                },
            };
            ObjectId conflictingHost = Id();
            Assert.Throws<BattlementUiException>(() =>
                world.Install(
                    new VisualElement(),
                    conflictingHost,
                    RoundTrip(Descriptor(conflictingHost, clock, factor, changedValues, 1))
                )
            );
        }

        [Test]
        public void FocusContributionReturnsToTheCurrentAuthoredScale()
        {
            ObjectId host = Id(),
                clock = Id(),
                factor = Id();
            var target = new VisualElement();
            target.style.scale = new Scale(Vector2.one);
            var values = new[]
            {
                new MotionValueDescriptor(
                    factor,
                    new MotionValue.Scalar(1.02),
                    new MotionValueSource.Mutable()
                ),
            };
            var descriptor = Descriptor(host, clock, factor, values, 1.1);
            descriptor = descriptor with
            {
                Slots = new[] { descriptor.Slots[0] with { Layer = MotionLayer.FocusVisible } },
            };
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, host, descriptor);
            world.PreLayout();
            world.PostLayout();
            world.SetFocusVisible(host, true);
            Sample(world, clock);
            AssertScale(target, 1.1 * 1.02);
            world.SetFocusVisible(host, false);
            Sample(world, clock);
            AssertScale(target, 1.02);
            BattlementPreparedMotionAdmission prepared = world.Prepare(
                target,
                host,
                descriptor with
                {
                    Generation = 2,
                    Slots = new[] { descriptor.Slots[0] with { Generation = 2 } },
                }
            )!;
            BattlementUiElementProperties.ApplyStyle(
                target,
                new UiStyle(Scale: UiStyle.Set(new UiScale(2, 2))),
                null,
                null,
                null,
                null
            );
            prepared.Commit();
            world.PreLayout();
            world.PostLayout();
            AssertScale(target, 2 * 1.02);
        }

        [Test]
        public void AxisAnimationUsesTheLocalScaleAndRestoresItAfterUnbinding()
        {
            var target = new VisualElement();
            target.style.scale = new Scale(new Vector2(2, 3));
            var pulse = new MotionValue.Vector2(new double[] { 1.1, 1.1 });
            BattlementMotionContributions.Set(target, MotionProperty.Scale, pulse);
            BattlementMotionPropertyWriter.WriteScalar(target, MotionProperty.ScaleX, 4);
            BattlementMotionPropertyWriter.WriteScalar(target, MotionProperty.ScaleY, 5);
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(4.4).Within(0.0001));
            Assert.That(target.style.scale.value.value.y, Is.EqualTo(5.5).Within(0.0001));
            var local = (MotionValue.Scalar)
                BattlementMotionPropertyWriter.Read(target, MotionProperty.ScaleX);
            Assert.That(local.Value, Is.EqualTo(4));
            BattlementMotionContributions.Set(target, MotionProperty.Scale, pulse);
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(4.4).Within(0.0001));
            BattlementMotionContributions.Remove(target, MotionProperty.Scale);
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(4));
            Assert.That(target.style.scale.value.value.y, Is.EqualTo(5));
        }

        private static MotionDescriptor RoundTrip(MotionDescriptor descriptor)
        {
            SessionId session = new(Guid.NewGuid());
            var response = new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(
                            new BatchId(Guid.NewGuid()),
                            session,
                            new[]
                            {
                                new ParallelCommandGroup<Command>(
                                    new[]
                                    {
                                        new Command(
                                            new CommandId(Guid.NewGuid()),
                                            new CommandBody.VisualElement.Update(
                                                new VisualElementUpdate.Properties(
                                                    descriptor.HostId,
                                                    new UiElement.Box { Motion = descriptor }
                                                )
                                            )
                                        ),
                                    }
                                ),
                            }
                        )
                    ),
                }
            );
            var decoded = BattlementJson.DeserializeResponse(
                BattlementJson.SerializeResponse(response)
            );
            var batch = (ResponseMessage<Command>.BatchMessage)decoded.Messages[0];
            var update = (CommandBody.VisualElement.Update)batch.Batch.Groups[0].Commands[0].Body;
            var properties = (VisualElementUpdate.Properties)update.Value;
            return properties.Element.Motion.Value;
        }

        private static void Sample(BattlementMotionWorld world, ObjectId clock)
        {
            world.AdvanceControlledClock(clock, 1_000_000);
            world.PreLayout();
            world.PostLayout();
        }

        private static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId clock,
            ObjectId factor,
            MotionValueDescriptor[] values,
            double scale
        ) =>
            new(
                host,
                host,
                1,
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        1,
                        1,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.Scale,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Vector2(new[] { scale, scale }),
                                    },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            1_000_000,
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
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.User,
                Values: values,
                ValueBindings: new[]
                {
                    new MotionValueBinding(
                        MotionProperty.Scale,
                        factor,
                        MotionBindingComposition.Compose
                    ),
                }
            );

        private static ObjectId Id() => new(Guid.NewGuid());

        private static void AssertScale(VisualElement target, double expected) =>
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(expected).Within(0.00001));
    }
}
