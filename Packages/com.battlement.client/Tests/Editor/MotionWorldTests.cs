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
        public void DocumentMotionKeepsScaledAndUnscaledClocksDistinct()
        {
            ObjectId host = Id("c857700c-8af5-4e2d-8516-061269d5660b");
            ObjectId descriptor = Id("3c2f1e09-7581-443e-86dc-6c73b8092174");
            ObjectId clock = Id("87cf3f1c-1d29-4f16-8234-72a763c3bb92");
            TimeSpan unscaled = TimeSpan.FromSeconds(10);
            TimeSpan scaled = TimeSpan.FromSeconds(0.25);
            var target = new VisualElement();
            target.style.opacity = 0;
            using var documents = new BattlementUiDocuments(
                now: () => unscaled,
                scaledNow: () => scaled
            );
            BattlementMotionWorld world = documents.MotionWorldForTests;
            world.Install(
                target,
                host,
                Descriptor(descriptor, host, clock, 1, 1, 1) with
                {
                    Clock = new MotionClockSource.Scaled(),
                }
            );

            unscaled = TimeSpan.FromSeconds(30);
            scaled = TimeSpan.FromSeconds(0.75);
            world.PostLayout();

            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
        }

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
        public void UserReducedMotionRespondsLiveAndKeepsNonSpatialTiming()
        {
            ObjectId clock = Id("65c0d0a2-ad0a-46df-b934-54a03f560257");
            ObjectId host = Id("259ace0f-8fd7-49e1-b401-41aec827926a");
            bool reduced = false;
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                reducedMotion: () => reduced
            );
            world.Install(target, host, ReducedDescriptor(host, clock));

            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(50).Within(0.001));
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.001));

            reduced = true;
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(100).Within(0.001));
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.001));

            reduced = false;
            world.SetControlledClock(clock, 750_000);
            world.PostLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(75).Within(0.001));
            Assert.That(target.style.opacity.value, Is.EqualTo(0.75f).Within(0.001));
        }

        [Test]
        public void ReducedMotionSuppressesSpatialGraphBindingsWithoutStoppingTheirValues()
        {
            ObjectId host = Id("af32cb17-bf35-419f-b5f0-ec014a833231");
            ObjectId value = Id("8768ea9c-ee59-410b-86cf-458077c1b7ee");
            bool reduced = false;
            var target = new VisualElement();
            using var world = new BattlementMotionWorld(
                registerPlayerLoop: false,
                reducedMotion: () => reduced
            );
            world.Install(
                target,
                host,
                GraphDescriptor(
                    host,
                    new[]
                    {
                        new MotionValueDescriptor(
                            value,
                            new MotionValue.Scalar(72),
                            new MotionValueSource.Mutable()
                        ),
                    },
                    new[] { new MotionValueBinding(MotionProperty.X, value) }
                ) with
                {
                    ReducedMotion = ReducedMotionPolicy.User,
                }
            );

            world.PreLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(72).Within(0.001));
            reduced = true;
            world.PreLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.Zero.Within(0.001));
            reduced = false;
            world.PreLayout();
            Assert.That(ReadPixels(target, MotionProperty.X), Is.EqualTo(72).Within(0.001));
        }

        [Test]
        public void ReconnectRestoresPhaseWithoutCancellationAndCleansMissingHosts()
        {
            ObjectId clock = Id("f34010c9-77dc-4da8-a7a8-a33a01289c5b");
            ObjectId host = Id("a5bda971-69dc-4fa1-b6a0-827ca36bd230");
            ObjectId staleHost = Id("d373cb08-e946-45cc-900a-89a030a02a2a");
            MotionDescriptor descriptor = Descriptor(host, host, clock, 1, 1, 1);
            var original = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(original, host, descriptor);
            world.Install(
                new VisualElement(),
                staleHost,
                Descriptor(staleHost, staleHost, clock, 1, 1, 1)
            );
            world.SetControlledClock(clock, 400_000);
            world.PostLayout();
            world.DrainEventBatch();
            float presentation = original.style.opacity.value;

            var replacement = new VisualElement();
            world.BeginReconnect();
            world.Install(replacement, host, descriptor);
            world.EndReconnect();

            Assert.That(world.DescriptorCount, Is.EqualTo(1));
            Assert.That(replacement.style.opacity.value, Is.EqualTo(presentation).Within(0.001));
            MotionEventBatch? reconnectEvents = world.DrainEventBatch();
            Assert.That(
                reconnectEvents?.Events.Any(value => value.Kind is MotionEventKind.Cancelled),
                Is.Not.True
            );

            world.SetControlledClock(clock, 1_000_000);
            world.PostLayout();
            Assert.That(replacement.style.opacity.value, Is.EqualTo(1).Within(0.001));
            MotionEventBatch completed = world.DrainEventBatch()!;
            Assert.That(
                completed.Events.Count(value => value.Kind is MotionEventKind.Completed),
                Is.EqualTo(1)
            );
        }

        [Test]
        public void ReconnectCancelsASequenceWhoseSnapshottedTargetWasRemoved()
        {
            ObjectId clock = Id("d17f15a2-9754-47cd-a72f-65a44de96140");
            ObjectId scope = Id("d17f15a2-9754-47cd-a72f-65a44de96141");
            ObjectId child = Id("d17f15a2-9754-47cd-a72f-65a44de96142");
            ObjectId playback = Id("d17f15a2-9754-47cd-a72f-65a44de96143");
            var root = new VisualElement();
            var target = new VisualElement();
            root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            MotionDescriptor rootDescriptor = EmptyDescriptor(scope, clock) with
            {
                ScopeId = scope,
                ScopeRoot = true,
            };
            world.Install(root, scope, rootDescriptor);
            world.Install(target, child, EmptyDescriptor(child, clock));
            world.Apply(
                new MotionScopeOperation(
                    scope,
                    new MotionScopeCommand.Start(
                        playback,
                        1,
                        new MotionSequenceEntry[]
                        {
                            Animate(Target(1, 1_000_000), new MotionSequenceSchedule.Absolute(0)),
                        }
                    )
                )
            );

            world.BeginReconnect();
            world.Install(new VisualElement(), scope, rootDescriptor);
            world.EndReconnect();

            MotionPlaybackEvent cancelled = world
                .DrainEventBatch()!
                .PlaybackEvents.Single(value => value.PlaybackId == playback);
            Assert.That(cancelled.Outcome, Is.EqualTo(MotionPlaybackOutcome.Cancelled));
            world.SetControlledClock(clock, 1_000_000);
            Assert.DoesNotThrow(world.PostLayout);
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
                CompoundDescriptor(host, clock, 1, new Color(0.1, 0.2, 0.3, 1))
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
                RetargetDescriptor(host, clock, 1, 1.5, new Color(0.1, 0.8, 0.9, 1))
            );
            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            float scale = target.style.scale.value.value.x;
            UnityEngine.Color color = target.style.backgroundColor.value;

            world.Install(
                target,
                host,
                RetargetDescriptor(host, clock, 2, 0.7, new Color(0.95, 0.3, 0.1, 1))
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
                    documents.Replace(
                        new[] { description },
                        id => id == document ? owned : null,
                        preserveMotion: true
                    )
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
            Assert.That(world.ActiveFiniteTimelineCount, Is.EqualTo(1));
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
            Assert.That(world.ActiveFiniteTimelineCount, Is.Zero);
            MotionPlaybackEvent completed = world.DrainEventBatch()!.PlaybackEvents.Single();
            Assert.That(completed.PlaybackId, Is.EqualTo(second));
            Assert.That(completed.Outcome, Is.EqualTo(MotionPlaybackOutcome.Completed));
            world.PreLayout();
            Assert.That(world.DrainEventBatch(), Is.Null);
        }

        [Test]
        public void TimeDerivedMotionValuesAreClassifiedAsContinuousMotion()
        {
            ObjectId host = Id("1aff84da-9715-460f-a88a-a5352b39a67b");
            ObjectId value = Id("a5fd1b9f-e1ef-4da4-b5dc-0bf09407cdcb");
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                new VisualElement(),
                host,
                GraphDescriptor(
                    host,
                    new[]
                    {
                        new MotionValueDescriptor(
                            value,
                            new MotionValue.Scalar(0),
                            new MotionValueSource.Time(new MotionClockSource.Unscaled())
                        ),
                    },
                    new[] { new MotionValueBinding(MotionProperty.Opacity, value) }
                )
            );

            Assert.That(world.ActiveInfiniteTimelineCount, Is.EqualTo(1));
            Assert.That(world.ActiveFiniteTimelineCount, Is.Zero);
        }

        [Test]
        public void ControlsAttachLateAndScopeSelectorsUseCommandTimeSnapshots()
        {
            ObjectId clock = Id("bce2847e-981e-4647-b8bd-7442cc000040");
            ObjectId control = Id("bce2847e-981e-4647-b8bd-7442cc000041");
            ObjectId scope = Id("bce2847e-981e-4647-b8bd-7442cc000042");
            ObjectId playback = Id("bce2847e-981e-4647-b8bd-7442cc000043");
            var controlled = new VisualElement();
            controlled.style.opacity = 0;
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
                            new MotionSequenceEntry.Animate(
                                new MotionSelector.Children(),
                                Target(1, 1_000_000),
                                null,
                                LinearTween(),
                                new MotionSequenceSchedule.Absolute(0),
                                MotionSequenceConflict.Reject
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

        [Test]
        public void SequenceLabelsUseActualCompletionAndPauseFutureScheduling()
        {
            ObjectId clock = Id("61b86de9-0408-4f43-9be7-e58f56cb7061");
            ObjectId scope = Id("61b86de9-0408-4f43-9be7-e58f56cb7062");
            ObjectId child = Id("61b86de9-0408-4f43-9be7-e58f56cb7063");
            ObjectId playback = Id("61b86de9-0408-4f43-9be7-e58f56cb7064");
            var root = new VisualElement();
            var target = new VisualElement();
            root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                root,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            world.Install(target, child, EmptyDescriptor(child, clock));
            world.Apply(
                new MotionScopeOperation(
                    scope,
                    new MotionScopeCommand.Start(
                        playback,
                        1,
                        new MotionSequenceEntry[]
                        {
                            Animate(Target(1, 400_000), new MotionSequenceSchedule.Absolute(0)),
                            new MotionSequenceEntry.Label(
                                "arrived",
                                new MotionSequenceSchedule.AfterCompletion(0, 0)
                            ),
                            Animate(
                                XTarget(20, 100_000),
                                new MotionSequenceSchedule.Absolute(100_000)
                            ),
                            new MotionSequenceEntry.Label(
                                "same-time-a",
                                new MotionSequenceSchedule.Absolute(100_000)
                            ),
                            new MotionSequenceEntry.Label(
                                "same-time-b",
                                new MotionSequenceSchedule.Absolute(100_000)
                            ),
                        }
                    )
                )
            );

            world.SetControlledClock(clock, 100_000);
            world.PostLayout();
            CollectionAssert.AreEqual(
                new[] { "same-time-a", "same-time-b" },
                world.DrainEventBatch()!.LabelEvents!.Select(value => value.Label).ToArray()
            );

            world.SetControlledClock(clock, 200_000);
            world.PostLayout();
            world.Apply(
                new MotionValuePlaybackOperation(playback, 1, new MotionPlaybackCommand.Pause())
            );
            world.SetControlledClock(clock, 700_000);
            world.PostLayout();
            Assert.That(world.DrainEventBatch(), Is.Null);

            world.Apply(
                new MotionValuePlaybackOperation(playback, 1, new MotionPlaybackCommand.Play())
            );
            world.SetControlledClock(clock, 900_000);
            world.PostLayout();
            MotionEventBatch completed = world.DrainEventBatch()!;
            Assert.That(completed.LabelEvents!.Single().Label, Is.EqualTo("arrived"));
            Assert.That(
                completed.PlaybackEvents!.Single().Outcome,
                Is.EqualTo(MotionPlaybackOutcome.Completed)
            );
        }

        [Test]
        public void SequenceEffectsValidateAtomicallyAndStartOnceInDeclarationOrder()
        {
            ObjectId clock = Id("71b86de9-0408-4f43-9be7-e58f56cb7061");
            ObjectId scope = Id("71b86de9-0408-4f43-9be7-e58f56cb7062");
            ObjectId playback = Id("71b86de9-0408-4f43-9be7-e58f56cb7063");
            var root = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            var effects = new RecordingEffects();
            world.BindEffects(effects);
            world.Install(
                root,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            MotionSequenceEntry[] entries =
            {
                new MotionSequenceEntry.Sound(
                    new MotionSoundOccurrence("audio/one", 1, 1, false, 0),
                    new MotionSequenceSchedule.Absolute(100)
                ),
                new MotionSequenceEntry.Sound(
                    new MotionSoundOccurrence("audio/two", 0.5, 1.2, false, 10),
                    new MotionSequenceSchedule.Absolute(100)
                ),
            };
            world.Apply(
                new MotionScopeOperation(scope, new MotionScopeCommand.Start(playback, 1, entries))
            );
            Assert.That(effects.RetainedPreparations, Is.EqualTo(2));
            world.SetControlledClock(clock, 100);
            world.PostLayout();
            Assert.That(effects.RetainedPreparations, Is.Zero);
            CollectionAssert.AreEqual(new[] { "audio/one", "audio/two" }, effects.Started);
            CollectionAssert.AreEqual(
                new[] { "audio/one", "audio/two" },
                world.EffectOccurrences.Select(value => value.Address).ToArray()
            );
            world.PostLayout();
            CollectionAssert.AreEqual(new[] { "audio/one", "audio/two" }, effects.Started);
            Assert.That(world.EffectOccurrences.Count, Is.EqualTo(2));

            effects.FailAddress = "audio/missing";
            Assert.Throws<BattlementUiException>(() =>
                world.Apply(
                    new MotionScopeOperation(
                        scope,
                        new MotionScopeCommand.Start(
                            Id("71b86de9-0408-4f43-9be7-e58f56cb7064"),
                            1,
                            new MotionSequenceEntry[]
                            {
                                new MotionSequenceEntry.Sound(
                                    new MotionSoundOccurrence("audio/valid", 1, 1, false, 0),
                                    new MotionSequenceSchedule.Absolute(0)
                                ),
                                new MotionSequenceEntry.Sound(
                                    new MotionSoundOccurrence("audio/missing", 1, 1, false, 0),
                                    new MotionSequenceSchedule.Absolute(0)
                                ),
                            }
                        )
                    )
                )
            );
            CollectionAssert.DoesNotContain(effects.Started, "audio/valid");
            Assert.That(effects.RetainedPreparations, Is.Zero);
        }

        [Test]
        public void SequenceParticlesCaptureAtSubmissionOrFollowAtOccurrence()
        {
            ObjectId clock = Id("61b86de9-0408-4f43-9be7-e58f56cb7061");
            ObjectId scope = Id("61b86de9-0408-4f43-9be7-e58f56cb7062");
            ObjectId target = Id("61b86de9-0408-4f43-9be7-e58f56cb7063");
            var root = new VisualElement();
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            var effects = new RecordingEffects { Position = new UnityEngine.Vector3(1, 2, 3) };
            world.BindEffects(effects);
            world.Install(
                root,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            world.Apply(
                new MotionScopeOperation(
                    scope,
                    new MotionScopeCommand.Start(
                        Id("61b86de9-0408-4f43-9be7-e58f56cb7064"),
                        1,
                        new MotionSequenceEntry[]
                        {
                            Particle(MotionReferenceResolution.CaptureAtStart),
                            Particle(MotionReferenceResolution.Follow),
                        }
                    )
                )
            );

            effects.Position = new UnityEngine.Vector3(4, 5, 6);
            world.SetControlledClock(clock, 100);
            world.PostLayout();

            CollectionAssert.AreEqual(
                new[] { new UnityEngine.Vector3(1, 2, 3), new UnityEngine.Vector3(4, 5, 6) },
                effects.ParticlePositions
            );

            MotionSequenceEntry.Particle Particle(MotionReferenceResolution resolution) =>
                new(
                    new MotionParticleOccurrence(
                        "particle/burst",
                        new MotionPositionReference(
                            target,
                            null,
                            Battlement.Vector3.Zero,
                            resolution
                        ),
                        250
                    ),
                    new MotionSequenceSchedule.Absolute(100)
                );
        }

        [Test]
        public void SequenceOwnershipSurvivesRetargetAndReturnsWithoutJump()
        {
            ObjectId clock = Id("20f2eddf-ffda-4cc6-bbc2-01a2f6eca715");
            ObjectId scope = Id("20f2eddf-ffda-4cc6-bbc2-01a2f6eca716");
            ObjectId child = Id("20f2eddf-ffda-4cc6-bbc2-01a2f6eca717");
            ObjectId playback = Id("20f2eddf-ffda-4cc6-bbc2-01a2f6eca718");
            var root = new VisualElement();
            var target = new VisualElement();
            target.style.opacity = 0;
            root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                root,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            world.Install(
                target,
                child,
                Descriptor(child, child, clock, 1, 1, 1, subscribe: false)
            );
            world.Apply(
                new MotionScopeOperation(
                    scope,
                    new MotionScopeCommand.Start(
                        playback,
                        1,
                        new[]
                        {
                            Animate(Target(1, 1_000_000), new MotionSequenceSchedule.Absolute(0)),
                        }
                    )
                )
            );

            world.SetControlledClock(clock, 500_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));

            world.Install(
                target,
                child,
                Descriptor(child, child, clock, 2, 2, 1, subscribe: false)
            );
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
            world.SetControlledClock(clock, 750_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.75f).Within(0.00001));

            world.SetControlledClock(clock, 1_000_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1).Within(0.00001));
            world.SetControlledClock(clock, 1_100_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(1).Within(0.00001));

            world.Install(
                target,
                child,
                Descriptor(child, child, clock, 3, 3, 0, subscribe: false)
            );
            world.SetControlledClock(clock, 1_600_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.EqualTo(0.5f).Within(0.00001));
        }

        [Test]
        public void InvalidSequenceLeavesPresentationAndPlaybackRegistryUntouched()
        {
            ObjectId clock = Id("f08b3f97-889f-4456-a973-c13551113f11");
            ObjectId scope = Id("f08b3f97-889f-4456-a973-c13551113f12");
            ObjectId child = Id("f08b3f97-889f-4456-a973-c13551113f13");
            var root = new VisualElement();
            var target = new VisualElement();
            target.style.opacity = 0;
            root.Add(target);
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                root,
                scope,
                EmptyDescriptor(scope, clock) with
                {
                    ScopeId = scope,
                    ScopeRoot = true,
                }
            );
            world.Install(target, child, EmptyDescriptor(child, clock));

            Assert.Throws<BattlementUiException>(() =>
                world.Apply(
                    new MotionScopeOperation(
                        scope,
                        new MotionScopeCommand.Start(
                            Id("f08b3f97-889f-4456-a973-c13551113f14"),
                            1,
                            new MotionSequenceEntry[]
                            {
                                Animate(
                                    Target(0.5, 400_000),
                                    new MotionSequenceSchedule.Absolute(0)
                                ),
                                Animate(Target(1, 400_000), new MotionSequenceSchedule.Absolute(0)),
                            }
                        )
                    )
                )
            );
            world.SetControlledClock(clock, 200_000);
            world.PostLayout();
            Assert.That(target.style.opacity.value, Is.Zero);
            Assert.That(world.DrainEventBatch(), Is.Null);
        }

        [Test]
        public void PerformanceSnapshotCountsWorkWithoutLifecycleTraffic()
        {
            ObjectId clock = Id("63dbdaeb-8152-4280-bbbd-76994880438d");
            ObjectId host = Id("9e75e297-ecbc-4332-87f7-bf9372220d2a");
            using var world = new BattlementMotionWorld(registerPlayerLoop: false);
            world.Install(
                new VisualElement(),
                host,
                Descriptor(host, host, clock, 1, 1, 1, subscribe: false)
            );

            world.SetControlledClock(clock, 500_000);
            world.PreLayout();
            world.PostLayout();

            BattlementMotionPerformanceSnapshot snapshot = world.Performance;
            Assert.That(snapshot.Frame, Is.EqualTo(1));
            Assert.That(snapshot.ActiveTimelines, Is.EqualTo(1));
            Assert.That(snapshot.ActiveLayoutTracks, Is.Zero);
            Assert.That(snapshot.PropertiesApplied, Is.EqualTo(1));
            Assert.That(snapshot.GraphNodesEvaluated, Is.Zero);
            Assert.That(snapshot.LifecycleMessages, Is.Zero);
            Assert.That(snapshot.LifecyclePayloadBytes, Is.Zero);
            Assert.That(snapshot.MotionCpuMilliseconds, Is.GreaterThanOrEqualTo(0));
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

        private static MotionDescriptor ReducedDescriptor(ObjectId host, ObjectId clock) =>
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
                                    MotionProperty.X,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Length(UiLength.FromComponents(100, 0)),
                                    },
                                    LinearTween()
                                ),
                                new MotionPropertyTrack(
                                    MotionProperty.Opacity,
                                    new MotionValue[]
                                    {
                                        new MotionValue.Scalar(0),
                                        new MotionValue.Scalar(1),
                                    },
                                    LinearTween()
                                ),
                            },
                            Array.Empty<MotionPropertyValue>()
                        ),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.User
            );

        private static TransitionDefinition LinearTween() =>
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

        private static float ReadPixels(VisualElement target, MotionProperty property) =>
            BattlementMotionPropertyWriter.Read(target, property) is MotionValue.Length value
                ? (float)value.Value.Pixels
                : 0;

        private static MotionDescriptor CompoundDescriptor(
            ObjectId host,
            ObjectId clock,
            uint generation,
            Color color
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
            Color color
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

        private static MotionTargetDescriptor XTarget(double x, ulong duration) =>
            new(
                new[]
                {
                    new MotionPropertyTrack(
                        MotionProperty.X,
                        new MotionValue[] { new MotionValue.Length(new UiLength.Px((float)x)) },
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

        private static MotionSequenceEntry.Animate Animate(
            MotionTargetDescriptor target,
            MotionSequenceSchedule schedule
        ) =>
            new(
                new MotionSelector.Children(),
                target,
                null,
                LinearTween(),
                schedule,
                MotionSequenceConflict.Reject
            );

        private static ObjectId Id(string value) => new(Guid.Parse(value));

        private sealed class RecordingEffects : IBattlementMotionEffects
        {
            public List<string> Started { get; } = new();
            public List<UnityEngine.Vector3> ParticlePositions { get; } = new();
            public string? FailAddress { get; set; }
            public UnityEngine.Vector3 Position { get; set; }
            public int RetainedPreparations { get; private set; }

            public IBattlementPreparedMotionEffect Prepare(MotionSequenceEntry entry)
            {
                string address = entry switch
                {
                    MotionSequenceEntry.Sound value => value.Occurrence.Address,
                    MotionSequenceEntry.Particle value => value.Occurrence.Address,
                    _ => throw new InvalidOperationException(),
                };
                if (address == FailAddress)
                    throw new BattlementUiException(
                        CoreErrorCode.AssetNotPrepared,
                        "effect asset is absent"
                    );
                RetainedPreparations++;
                return new RecordingPreparedEffect(() => RetainedPreparations--);
            }

            public UnityEngine.Vector3 Resolve(MotionPositionReference reference) => Position;

            public void Start(
                ObjectId playbackId,
                int entryIndex,
                MotionSequenceEntry entry,
                IBattlementPreparedMotionEffect prepared,
                UnityEngine.Vector3? capturedPosition
            )
            {
                ((RecordingPreparedEffect)prepared).Consume();
                Started.Add(
                    entry switch
                    {
                        MotionSequenceEntry.Sound value => value.Occurrence.Address,
                        MotionSequenceEntry.Particle value => value.Occurrence.Address,
                        _ => throw new InvalidOperationException(),
                    }
                );
                if (entry is MotionSequenceEntry.Particle particle)
                    ParticlePositions.Add(
                        capturedPosition ?? Resolve(particle.Occurrence.Position)
                    );
            }

            public void Advance() { }

            public void Reset() { }

            public void Dispose() { }

            private sealed class RecordingPreparedEffect : IBattlementPreparedMotionEffect
            {
                private System.Action? release;

                public RecordingPreparedEffect(System.Action release) => this.release = release;

                public void Consume()
                {
                    Assert.That(release, Is.Not.Null);
                    Dispose();
                }

                public void Dispose()
                {
                    release?.Invoke();
                    release = null;
                }
            }
        }
    }
}
