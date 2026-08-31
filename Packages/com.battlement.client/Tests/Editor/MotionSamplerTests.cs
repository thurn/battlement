#nullable enable

using System;
using System.IO;
using Battlement.UI;
using Newtonsoft.Json.Linq;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class MotionSamplerTests
    {
        private const string VectorsPath =
            "Packages/com.battlement.client/Tests/Editor/Fixtures/Motion/motion-13.1.1.json";

        [Test]
        public void ScalarSamplerMatchesPinnedMotionVectors()
        {
            JObject fixture = JObject.Parse(File.ReadAllText(VectorsPath));
            Assert.That((string?)fixture["source"], Is.EqualTo("motion@13.1.1"));
            double tolerance = (double)fixture["tolerance"]!;
            foreach (JToken vector in fixture["vectors"]!)
            {
                MotionScalarSample sample = BattlementMotionScalarSampler.Sample(
                    0,
                    1,
                    0,
                    Transition((string)vector["kind"]!),
                    (ulong)vector["elapsed_micros"]!
                );
                Assert.That(sample.Value, Is.EqualTo((double)vector["value"]!).Within(tolerance));
                Assert.That(
                    sample.Velocity,
                    Is.EqualTo((double)vector["velocity"]!).Within(tolerance)
                );
            }
        }

        [Test]
        public void RepeatNegativeDelayAndDiscreteSegmentsUseLogicalTime()
        {
            TransitionDefinition repeated = Tween(
                delay: -250_000,
                repeat: new MotionRepeat.Count(1),
                repeatType: MotionRepeatType.Reverse
            );
            Assert.That(Sample(repeated, 0).Value, Is.EqualTo(0.25).Within(0.00001));
            Assert.That(Sample(repeated, 1_250_000).Value, Is.EqualTo(0.5).Within(0.00001));

            var track = new MotionPropertyTrack(
                MotionProperty.Visibility,
                new MotionValue[]
                {
                    new MotionValue.Discrete(JToken.FromObject("visible")),
                    new MotionValue.Discrete(JToken.FromObject("hidden")),
                },
                Tween()
            );
            MotionValue before = BattlementMotionValueSampler
                .Sample(track, track.Values[0], 0, 499_999)
                .Value;
            MotionValue after = BattlementMotionValueSampler
                .Sample(track, track.Values[0], 0, 500_000)
                .Value;
            Assert.That(
                ((MotionValue.Discrete)before).Value.Value<string>(),
                Is.EqualTo("visible")
            );
            Assert.That(((MotionValue.Discrete)after).Value.Value<string>(), Is.EqualTo("hidden"));
        }

        [Test]
        public void DirectRepeatedBackwardAndSteppedSamplingAreEquivalent()
        {
            TransitionDefinition transition = Tween();
            MotionScalarSample direct = Sample(transition, 730_000);
            MotionScalarSample repeated = Sample(transition, 730_000);
            MotionScalarSample backward = Sample(transition, 730_000);
            MotionScalarSample stepped = default;
            for (ulong elapsed = 0; elapsed <= 730_000; elapsed += 10_000)
                stepped = Sample(transition, elapsed);
            Assert.That(repeated.Value, Is.EqualTo(direct.Value));
            Assert.That(backward.Value, Is.EqualTo(direct.Value));
            Assert.That(stepped.Value, Is.EqualTo(direct.Value));
            Assert.That(stepped.Velocity, Is.EqualTo(direct.Velocity));
        }

        [Test]
        public void SteadyScalarSamplingAllocatesNoManagedMemory()
        {
            TransitionDefinition transition = Tween();
            for (int index = 0; index < 100; index++)
                _ = Sample(transition, (ulong)index * 1_000);
            long before = GC.GetAllocatedBytesForCurrentThread();
            for (int index = 0; index < 10_000; index++)
                _ = Sample(transition, (ulong)(index % 1_000) * 1_000);
            Assert.That(GC.GetAllocatedBytesForCurrentThread() - before, Is.Zero);
        }

        private static MotionScalarSample Sample(
            TransitionDefinition transition,
            ulong elapsedMicros
        ) => BattlementMotionScalarSampler.Sample(0, 1, 0, transition, elapsedMicros);

        private static TransitionDefinition Transition(string kind) =>
            kind switch
            {
                "tween" => Tween(),
                "spring-under" => Spring(10),
                "spring-critical" => Spring(20),
                "spring-over" => Spring(30),
                "inertia" => new TransitionDefinition(
                    new TransitionGenerator.Inertia(
                        100,
                        0.8,
                        325_000,
                        null,
                        null,
                        0.5,
                        500,
                        10,
                        new InertiaTarget.Identity()
                    ),
                    0,
                    new MotionRepeat.None(),
                    0,
                    MotionRepeatType.Loop
                ),
                _ => throw new InvalidOperationException("Unknown conformance vector."),
            };

        private static TransitionDefinition Spring(double damping) =>
            new(
                new TransitionGenerator.Spring(
                    new SpringConfiguration.Physical(100, damping, 1, 0, 0, 0)
                ),
                0,
                new MotionRepeat.None(),
                0,
                MotionRepeatType.Loop
            );

        private static TransitionDefinition Tween(
            long delay = 0,
            MotionRepeat? repeat = null,
            MotionRepeatType repeatType = MotionRepeatType.Loop
        ) =>
            new(
                new TransitionGenerator.Tween(
                    1_000_000,
                    new MotionEasing[] { new MotionEasing.Linear() },
                    null
                ),
                delay,
                repeat ?? new MotionRepeat.None(),
                0,
                repeatType
            );
    }
}
