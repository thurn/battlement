#nullable enable

using System;
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

        private static ObjectId Id(string value) => new(Guid.Parse(value));
    }
}
