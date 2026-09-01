#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Battlement.UI;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.LowLevel;
using UnityEngine.UIElements;
using UnityPreLateUpdate = UnityEngine.PlayerLoop.PreLateUpdate;

namespace Battlement.Tests
{
    public sealed class MotionWorldTests
    {
        [Test]
        public void ControlledClockSamplesAndRetargetsFromVisiblePresentation()
        {
            ObjectId clock = Id("115dc154-b3bd-4b66-bf06-3236cad8db9f");
            ObjectId host = Id("ced67c7d-6788-4ae5-a8a3-c62e5585ce50");
            ObjectId descriptor = Id("17ac3c3f-100c-4d1d-8c15-d0a45bb91d2c");
            var target = new VisualElement();
            target.style.opacity = 0.2f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(target, host, Descriptor(descriptor, host, clock, 1, 1, 1));
            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.00001));

            world.Install(target, host, Descriptor(descriptor, host, clock, 2, 2, 0));
            Assert.That(target.style.opacity.value, Is.EqualTo(0.6f).Within(0.00001));
            world.AdvanceControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.3f).Within(0.00001));
        }

        [Test]
        public void KeyframeBoundaryStructuredDiscreteAndTransitionEndSampleTogether()
        {
            ObjectId clock = Id("d0961886-84a6-49cb-af9e-ea4e49dc6f26");
            ObjectId host = Id("99c8e61f-6458-43c4-94ff-26077c2d6bb1");
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                CompoundDescriptor(host, clock, 1, new MotionColor(0.1, 0.2, 0.3, 1))
            );
            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(1.25f).Within(0.00001));
            Assert.That(target.style.visibility.value, Is.EqualTo(Visibility.Hidden));

            world.SetControlledClock(clock, 1_000_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.7f).Within(0.00001));
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(1f).Within(0.00001));
            Assert.That(target.style.visibility.value, Is.EqualTo(Visibility.Visible));
        }

        [Test]
        public void TransformAndColorRetargetKeepTheVisiblePresentation()
        {
            ObjectId clock = Id("e0728f25-9769-401a-830b-3086692d09b4");
            ObjectId host = Id("a47dd89c-2543-4311-acd9-89ab47d6685b");
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                RetargetDescriptor(host, clock, 1, 1.5, new MotionColor(0.1, 0.8, 0.9, 1))
            );
            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            float scale = target.style.scale.value.value.x;
            UnityEngine.Color color = target.style.backgroundColor.value;

            world.Install(
                target,
                host,
                RetargetDescriptor(host, clock, 2, 0.7, new MotionColor(0.95, 0.3, 0.1, 1))
            );
            Assert.That(target.style.scale.value.value.x, Is.EqualTo(scale).Within(0.00001));
            Assert.That(target.style.backgroundColor.value, Is.EqualTo(color));
            world.AdvanceControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.backgroundColor.value.r, Is.GreaterThan(color.r));
        }

        [Test]
        public void RejectedGenerationPreservesPresentationAndInstalledDescriptor()
        {
            ObjectId clock = Id("2f46bfea-5fa8-443c-8c91-dade039200cc");
            ObjectId host = Id("866534f4-9da6-456f-86d8-087bd0b44209");
            ObjectId descriptor = Id("a0296869-3be9-43c4-a9be-73bddbb3792c");
            var target = new VisualElement();
            target.style.opacity = 0.25f;
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            MotionDescriptor accepted = Descriptor(descriptor, host, clock, 4, 8, 1);
            world.Install(target, host, accepted);
            world.SetControlledClock(clock, 400_000);
            world.PostLayout();
            float presentation = target.style.opacity.value;

            Assert.Throws<BattlementUiException>(() =>
                world.Install(target, host, Descriptor(descriptor, host, clock, 4, 8, 0))
            );
            Assert.That(world.DescriptorCount, Is.EqualTo(1));
            Assert.That(target.style.opacity.value, Is.EqualTo(presentation));
        }

        [Test]
        public void AuthoritativeReplacementCanRestoreTheSameGeneration()
        {
            ObjectId document = Id("cb55e4fc-9c13-4de2-8f65-42b63e59fec1");
            ObjectId root = Id("5a18e5d5-7ab9-434d-a71d-07b0f1b50145");
            ObjectId host = Id("8ed4c033-654b-4d89-a1eb-a54381695ff4");
            ObjectId descriptor = Id("2453eb5c-12a3-40da-bae3-a1c090052f40");
            ObjectId clock = Id("d0c42854-898f-45de-a63d-a93cc29c7c35");
            GameObject owned = BattlementUiDocuments.CreateGameObject(
                new GameObjectKind.UiDocumentState(root)
            );
            using var documents = new BattlementUiDocuments();
            try
            {
                var description = new UiDocument(
                    document,
                    root,
                    Children: new[]
                    {
                        new UiNode(
                            host,
                            new UiElement.Box
                            {
                                Motion = Descriptor(descriptor, host, clock, 4, 8, 1),
                            }
                        ),
                    }
                );

                documents.Replace(new[] { description }, id => id == document ? owned : null);
                Assert.DoesNotThrow(() =>
                    documents.Replace(new[] { description }, id => id == document ? owned : null)
                );
                Assert.That(documents.TryGet(host, out VisualElement? restored), Is.True);
                Assert.That(restored, Is.Not.Null);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(owned);
            }
        }

        [Test]
        public void MissedBoundariesAreCoalescedInOrderAndSeekSuppressesSideEffects()
        {
            ObjectId clock = Id("f36f891a-39f7-42a4-9ea7-90f4c74bdc8c");
            ObjectId host = Id("18f58ccf-30e5-4675-941f-c14fc4049ad9");
            ObjectId descriptor = Id("6cf57aa9-f6bd-4f1a-8ea5-96ab860b03dc");
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                Descriptor(descriptor, host, clock, 1, 3, 1, repeatCount: 3)
            );
            world.DrainEvents();
            world.SetControlledClock(clock, 3_500_000);
            world.PostLayout();
            MotionLifecycleEvent[] events = world.DrainEvents().ToArray();
            Assert.That(
                events.Select(value => value.Kind.GetType()),
                Is.EqualTo(
                    new[] { typeof(MotionEventKind.Started), typeof(MotionEventKind.Repeated) }
                )
            );
            var repeated = (MotionEventKind.Repeated)events[1].Kind;
            Assert.That((repeated.First, repeated.Last), Is.EqualTo((1u, 3u)));

            world.Seek(descriptor, 3, 1, 500_000);
            world.PostLayout();
            Assert.That(world.DrainEvents(), Is.Empty);
            Assert.That(world.DrainSamples().Count, Is.EqualTo(1));
            world.Seek(descriptor, 3, 1, 500_000);
            world.PostLayout();
            Assert.That(world.DrainEvents(), Is.Empty);
            Assert.That(world.DrainSamples().Count, Is.EqualTo(1));
            world.Seek(descriptor, 3, 1, 250_000);
            world.PostLayout();
            Assert.That(world.DrainEvents(), Is.Empty);
            Assert.That(world.DrainSamples().Count, Is.EqualTo(1));
        }

        [Test]
        public void PlayerLoopHasOnePanelUpdateWithAdjacentMotionPhasesAndRestores()
        {
            Assert.That(BattlementMotionPlayerLoop.IsInstalled, Is.False);
            using (var first = new BattlementMotionWorld())
            using (var second = new BattlementMotionWorld())
            {
                first.EnsurePlayerLoop();
                second.EnsurePlayerLoop();
                Assert.That(BattlementMotionPlayerLoop.IsInstalled, Is.True);
                Assert.That(BattlementMotionPlayerLoop.HasNormalizedTopology(), Is.True);
            }
            Assert.That(BattlementMotionPlayerLoop.IsInstalled, Is.False);
        }

        [Test]
        public void SteadyMotionWorldSamplingAllocatesNoManagedMemory()
        {
            ObjectId clock = Id("834f65c8-a845-4fd0-b0e2-f14e36e42b42");
            ObjectId host = Id("5bc72bee-a505-4494-9796-fd5aca781d3c");
            ObjectId descriptor = Id("27e5a10e-e103-4a5e-822e-43fce12f989d");
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                target,
                host,
                Descriptor(descriptor, host, clock, 1, 1, 1, subscribe: false)
            );
            world.DrainEvents();
            for (int index = 0; index < 100; index++)
            {
                world.SetControlledClock(clock, (ulong)index * 1_000);
                world.PostLayout();
            }
            long before = GC.GetAllocatedBytesForCurrentThread();
            for (int index = 0; index < 10_000; index++)
            {
                world.SetControlledClock(clock, (ulong)(index % 900) * 1_000);
                world.PostLayout();
            }
            Assert.That(GC.GetAllocatedBytesForCurrentThread() - before, Is.Zero);
        }

        [Test]
        public void DefaultPlayerLoopMatchesThePinnedReleaseTopology()
        {
            const string FixturePath =
                "Packages/com.battlement.client/Tests/Editor/Fixtures/Motion/"
                + "release-playerloop.json";
            JObject fixture = JObject.Parse(File.ReadAllText(FixturePath));
            PlayerLoopSystem parent = Find(
                PlayerLoop.GetDefaultPlayerLoop(),
                typeof(UnityPreLateUpdate)
            );
            Assert.That(Application.unityVersion, Is.EqualTo((string)fixture["unity_version"]!));
            Assert.That(
                parent.subSystemList.Select(value => value.type.FullName),
                Is.EqualTo(fixture["ordered_siblings"]!.Values<string>())
            );
        }

        [Test]
        public void SharedValueGraphEvaluatesOnlyDirtyNodesAndCoalescesSamples()
        {
            ObjectId source = Id("bce2847e-981e-4647-b8bd-7442cc000001");
            ObjectId derived = Id("bce2847e-981e-4647-b8bd-7442cc000002");
            ObjectId subscription = Id("bce2847e-981e-4647-b8bd-7442cc000003");
            MotionValueDescriptor[] values =
            {
                new(source, new MotionValue.Scalar(0.2), new MotionValueSource.Mutable()),
                new(
                    derived,
                    new MotionValue.Scalar(0.2),
                    new MotionValueSource.Range(
                        source,
                        new MotionValue[] { new MotionValue.Scalar(0), new MotionValue.Scalar(1) },
                        new MotionValue[] { new MotionValue.Scalar(0), new MotionValue.Scalar(1) },
                        true
                    )
                ),
            };
            var first = new VisualElement();
            var second = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                first,
                Id("bce2847e-981e-4647-b8bd-7442cc000010"),
                GraphDescriptor(
                    Id("bce2847e-981e-4647-b8bd-7442cc000010"),
                    values,
                    new[] { new MotionValueBinding(MotionProperty.Opacity, derived) },
                    new[]
                    {
                        new MotionValueSubscription(
                            subscription,
                            source,
                            MotionValueEventKind.Change
                        ),
                    }
                )
            );
            world.Install(
                second,
                Id("bce2847e-981e-4647-b8bd-7442cc000011"),
                GraphDescriptor(
                    Id("bce2847e-981e-4647-b8bd-7442cc000011"),
                    values,
                    new[] { new MotionValueBinding(MotionProperty.Opacity, derived) }
                )
            );

            world.PreLayout();
            Assert.That(world.GraphNodeCount, Is.EqualTo(2));
            Assert.That(world.LastGraphEvaluationCount, Is.EqualTo(2));
            world.DrainEventBatch();
            world.PreLayout();
            Assert.That(world.LastGraphEvaluationCount, Is.Zero);

            world.Apply(
                new MotionValueOperation(
                    source,
                    new MotionValueCommand.Set(new MotionValue.Scalar(0.4))
                )
            );
            world.PreLayout();
            world.Apply(
                new MotionValueOperation(
                    source,
                    new MotionValueCommand.Set(new MotionValue.Scalar(0.8))
                )
            );
            world.PreLayout();
            MotionValueSample[] samples = world.DrainEventBatch()!.ValueSamples.ToArray();
            Assert.That(samples, Has.Length.EqualTo(1));
            Assert.That(((MotionValue.Scalar)samples[0].Value).Value, Is.EqualTo(0.8));
            Assert.That(first.style.opacity.value, Is.EqualTo(0.8f).Within(0.00001));
            Assert.That(second.style.opacity.value, Is.EqualTo(0.8f).Within(0.00001));
        }

        [Test]
        public void ValueGraphRejectsCyclesBeforeChangingTheInstalledWorld()
        {
            ObjectId first = Id("bce2847e-981e-4647-b8bd-7442cc000020");
            ObjectId second = Id("bce2847e-981e-4647-b8bd-7442cc000021");
            MotionValueDescriptor[] values =
            {
                new(
                    first,
                    new MotionValue.Scalar(0),
                    new MotionValueSource.Expression(
                        new MotionExpressionOperation.Add(),
                        new[] { second, second }
                    )
                ),
                new(
                    second,
                    new MotionValue.Scalar(0),
                    new MotionValueSource.Expression(
                        new MotionExpressionOperation.Add(),
                        new[] { first, first }
                    )
                ),
            };
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            Assert.Throws<BattlementUiException>(() =>
                world.Install(
                    new VisualElement(),
                    first,
                    GraphDescriptor(first, values, Array.Empty<MotionValueBinding>())
                )
            );
            Assert.That(world.DescriptorCount, Is.Zero);
            Assert.That(world.GraphNodeCount, Is.Zero);
        }

        [Test]
        public void AudioClockFreezesAndMarksSeekAndReplacementDiscontinuities()
        {
            ObjectId playback = Id("bce2847e-981e-4647-b8bd-7442cc000030");
            ObjectId time = Id("bce2847e-981e-4647-b8bd-7442cc000031");
            ObjectId subscription = Id("bce2847e-981e-4647-b8bd-7442cc000032");
            ulong elapsed = 0;
            bool discontinuity = false;
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                audioTime: _ => new MotionClockSample(elapsed, discontinuity)
            );
            world.Install(
                new VisualElement(),
                time,
                GraphDescriptor(
                    time,
                    new[]
                    {
                        new MotionValueDescriptor(
                            time,
                            new MotionValue.Scalar(0),
                            new MotionValueSource.Time(new MotionClockSource.Audio(playback))
                        ),
                    },
                    Array.Empty<MotionValueBinding>(),
                    new[]
                    {
                        new MotionValueSubscription(
                            subscription,
                            time,
                            MotionValueEventKind.Change
                        ),
                    }
                )
            );
            world.PreLayout();
            world.DrainEventBatch();
            elapsed = 250_000;
            world.PreLayout();
            MotionValueSample moving = world.DrainEventBatch()!.ValueSamples.Single();
            Assert.That(((MotionValue.Scalar)moving.Value).Value, Is.EqualTo(0.25));
            Assert.That(((MotionValue.Scalar)moving.Velocity).Value, Is.EqualTo(1).Within(0.00001));

            world.PreLayout();
            Assert.That(world.DrainEventBatch(), Is.Null);
            elapsed = 800_000;
            discontinuity = true;
            world.PreLayout();
            MotionValueSample jumped = world.DrainEventBatch()!.ValueSamples.Single();
            Assert.That(jumped.Discontinuity, Is.True);
            Assert.That(((MotionValue.Scalar)jumped.Velocity).Value, Is.Zero);
        }

        [Test]
        public void ValuePlaybackReportsReplacementAndNaturalCompletionExactlyOnce()
        {
            ObjectId source = Id("bce2847e-981e-4647-b8bd-7442cc000033");
            ObjectId first = Id("bce2847e-981e-4647-b8bd-7442cc000034");
            ObjectId second = Id("bce2847e-981e-4647-b8bd-7442cc000035");
            double now = 0;
            using var world = new BattlementMotionWorld(
                unscaledTime: () => now,
                registerPlayerLoop: false
            );
            world.Install(
                new VisualElement(),
                source,
                GraphDescriptor(
                    source,
                    new[]
                    {
                        new MotionValueDescriptor(
                            source,
                            new MotionValue.Scalar(0),
                            new MotionValueSource.Mutable()
                        ),
                    },
                    Array.Empty<MotionValueBinding>()
                )
            );
            TransitionDefinition transition = new(
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
            world.Apply(
                new MotionValueOperation(
                    source,
                    new MotionValueCommand.Animate(
                        first,
                        1,
                        new MotionValue.Scalar(0.5),
                        transition
                    )
                )
            );
            world.Apply(
                new MotionValueOperation(
                    source,
                    new MotionValueCommand.Animate(second, 1, new MotionValue.Scalar(1), transition)
                )
            );
            MotionPlaybackEvent replaced = world.DrainEventBatch()!.PlaybackEvents.Single();
            Assert.That(replaced.PlaybackId, Is.EqualTo(first));
            Assert.That(replaced.Outcome, Is.EqualTo(MotionPlaybackOutcome.Cancelled));

            now = 1.1;
            world.PreLayout();
            MotionPlaybackEvent completed = world.DrainEventBatch()!.PlaybackEvents.Single();
            Assert.That(completed.PlaybackId, Is.EqualTo(second));
            Assert.That(completed.Outcome, Is.EqualTo(MotionPlaybackOutcome.Completed));
            world.PreLayout();
            Assert.That(world.DrainEventBatch(), Is.Null);
        }

        [Test]
        public void ControlsAttachLateAndScopeSelectorsUseCommandTimeSnapshots()
        {
            ObjectId clock = Id("bce2847e-981e-4647-b8bd-7442cc000040");
            ObjectId control = Id("bce2847e-981e-4647-b8bd-7442cc000041");
            ObjectId scope = Id("bce2847e-981e-4647-b8bd-7442cc000042");
            ObjectId playback = Id("bce2847e-981e-4647-b8bd-7442cc000043");
            var controlled = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Apply(
                new MotionControlOperation(
                    control,
                    new MotionControlCommand.Start(
                        playback,
                        1,
                        new MotionControlTarget.Target(Target(1, 1_000_000))
                    )
                )
            );
            world.Install(
                controlled,
                control,
                EmptyDescriptor(control, clock) with
                {
                    ControlId = control,
                }
            );
            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(controlled.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
            world.SetControlledClock(clock, 1_100_000);
            world.PostLayout();
            MotionPlaybackEvent completed = world
                .DrainEventBatch()!
                .PlaybackEvents.Single(value => value.PlaybackId == playback);
            Assert.That(completed.Outcome, Is.EqualTo(MotionPlaybackOutcome.Completed));

            var rootElement = new VisualElement();
            var selected = new VisualElement();
            var late = new VisualElement();
            rootElement.Add(selected);
            world.Install(
                rootElement,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            world.Install(
                selected,
                Id("bce2847e-981e-4647-b8bd-7442cc000044"),
                EmptyDescriptor(Id("bce2847e-981e-4647-b8bd-7442cc000044"), clock)
            );
            world.Apply(
                new MotionScopeOperation(
                    scope,
                    new MotionScopeCommand.Start(
                        Id("bce2847e-981e-4647-b8bd-7442cc000045"),
                        1,
                        new[]
                        {
                            new MotionSequenceStep(
                                new MotionSelector.Children(),
                                Target(1, 1_000_000),
                                0
                            ),
                        }
                    )
                )
            );
            rootElement.Add(late);
            world.Install(
                late,
                Id("bce2847e-981e-4647-b8bd-7442cc000046"),
                EmptyDescriptor(Id("bce2847e-981e-4647-b8bd-7442cc000046"), clock)
            );
            world.SetControlledClock(clock, 2_500_000);
            world.PostLayout();
            Assert.That(selected.style.opacity.value, Is.EqualTo(1).Within(0.00001));
            Assert.That(late.style.opacity.value, Is.EqualTo(0).Within(0.00001));
        }

        private static PlayerLoopSystem Find(PlayerLoopSystem parent, Type type)
        {
            if (parent.type == type)
                return parent;
            foreach (
                PlayerLoopSystem child in parent.subSystemList ?? Array.Empty<PlayerLoopSystem>()
            )
            {
                PlayerLoopSystem found = Find(child, type);
                if (found.type == type)
                    return found;
            }
            return default;
        }

        private static MotionDescriptor Descriptor(
            ObjectId descriptorId,
            ObjectId hostId,
            ObjectId clockId,
            uint descriptorGeneration,
            ulong slot,
            double target,
            uint repeatCount = 0,
            bool subscribe = true
        ) =>
            new(
                descriptorId,
                hostId,
                descriptorGeneration,
                new[]
                {
                    new MotionPropertyValue(MotionProperty.Opacity, new MotionValue.Scalar(0.2)),
                },
                false,
                new[]
                {
                    new MotionSlotDescriptor(
                        slot,
                        descriptorGeneration,
                        MotionLayer.Animate,
                        new MotionTargetDescriptor(
                            new[]
                            {
                                new MotionPropertyTrack(
                                    MotionProperty.Opacity,
                                    new MotionValue[] { new MotionValue.Scalar(target) },
                                    new TransitionDefinition(
                                        new TransitionGenerator.Tween(
                                            1_000_000,
                                            new MotionEasing[] { new MotionEasing.Linear() },
                                            null
                                        ),
                                        0,
                                        repeatCount == 0
                                            ? new MotionRepeat.None()
                                            : new MotionRepeat.Count(repeatCount),
                                        0,
                                        MotionRepeatType.Loop
                                    )
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(
                            subscribe,
                            subscribe,
                            subscribe,
                            subscribe,
                            false,
                            false
                        )
                    ),
                },
                new MotionClockSource.Controlled(clockId),
                ReducedMotionPolicy.Never,
                null
            );

        private static MotionDescriptor CompoundDescriptor(
            ObjectId host,
            ObjectId clock,
            uint generation,
            MotionColor color
        )
        {
            TransitionDefinition tween = new(
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
            return new MotionDescriptor(
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
                            new MotionPropertyTrack[]
                            {
                                new(
                                    MotionProperty.Opacity,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Scalar(0),
                                        new MotionValue.Scalar(0.8),
                                        new MotionValue.Scalar(0.2),
                                        new MotionValue.Scalar(1),
                                    },
                                    tween,
                                    new double[] { 0, 0.25, 0.75, 1 }
                                ),
                                new(
                                    MotionProperty.Scale,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Vector2(new double[] { 1, 1 }),
                                        new MotionValue.Vector2(new double[] { 1.25, 0.75 }),
                                        new MotionValue.Vector2(new double[] { 1, 1 }),
                                    },
                                    tween
                                ),
                                new(
                                    MotionProperty.Visibility,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Discrete("visible"),
                                        new MotionValue.Discrete("hidden"),
                                        new MotionValue.Discrete("visible"),
                                    },
                                    tween
                                ),
                                new(
                                    MotionProperty.BackgroundColor,
                                    new MotionValue[] { new MotionValue.Color(color) },
                                    tween
                                ),
                            },
                            new[]
                            {
                                new MotionPropertyValue(
                                    MotionProperty.Opacity,
                                    new MotionValue.Scalar(0.7)
                                ),
                            }
                        ),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );
        }

        private static MotionDescriptor RetargetDescriptor(
            ObjectId host,
            ObjectId clock,
            uint generation,
            double scale,
            MotionColor color
        )
        {
            TransitionDefinition tween = new(
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
            return new MotionDescriptor(
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
                            new MotionPropertyTrack[]
                            {
                                new(
                                    MotionProperty.Scale,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Vector2(new double[] { scale, scale }),
                                    },
                                    tween
                                ),
                                new(
                                    MotionProperty.BackgroundColor,
                                    new MotionValue[] { new MotionValue.Color(color) },
                                    tween
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );
        }

        private static MotionDescriptor GraphDescriptor(
            ObjectId host,
            IReadOnlyList<MotionValueDescriptor> values,
            IReadOnlyList<MotionValueBinding> bindings,
            IReadOnlyList<MotionValueSubscription>? subscriptions = null
        ) =>
            EmptyDescriptor(host, Id("bce2847e-981e-4647-b8bd-7442cc000099")) with
            {
                Values = values,
                ValueBindings = bindings,
                ValueSubscriptions = subscriptions,
            };

        private static MotionDescriptor EmptyDescriptor(ObjectId host, ObjectId clock) =>
            new(
                host,
                host,
                1,
                new[]
                {
                    new MotionPropertyValue(MotionProperty.Opacity, new MotionValue.Scalar(0)),
                },
                false,
                Array.Empty<MotionSlotDescriptor>(),
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );

        private static MotionTargetDescriptor Target(double opacity, ulong duration) =>
            new(
                new[]
                {
                    new MotionPropertyTrack(
                        MotionProperty.Opacity,
                        new MotionValue[] { new MotionValue.Scalar(opacity) },
                        new TransitionDefinition(
                            new TransitionGenerator.Tween(
                                duration,
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
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
