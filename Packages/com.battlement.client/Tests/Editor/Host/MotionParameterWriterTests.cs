#nullable enable

using System;
using System.Linq;
using Battlement.UI;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class MotionParameterWriterTests
    {
        [Test]
        public void LightAndParticleWritersSampleOneSharedTimeline()
        {
            var host = new GameObject("Motion parameter host");
            var particles = new GameObject("Motion particles");
            particles.transform.SetParent(host.transform, false);
            Light light = host.AddComponent<Light>();
            ParticleSystem particleSystem = particles.AddComponent<ParticleSystem>();
            try
            {
                light.intensity = 2;
                ParticleSystem.EmissionModule emission = particleSystem.emission;
                emission.rateOverTimeMultiplier = 4;
                ObjectId id = Id();
                ObjectId clock = Id();
                MotionDescriptor descriptor = Descriptor(
                    id,
                    clock,
                    Track(MotionProperty.LightIntensity, 6),
                    Track(MotionProperty.ParticleEmission, 12)
                );
                var target = new BattlementWorldMotionTarget(host.transform);
                target.Configure(descriptor);
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion.Prepare(target, id, descriptor)!.Commit();
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();

                Assert.That(light.intensity, Is.EqualTo(4).Within(0.00001));
                Assert.That(
                    particleSystem.emission.rateOverTimeMultiplier,
                    Is.EqualTo(8).Within(0.00001)
                );
            }
            finally
            {
                Object.DestroyImmediate(host);
            }
        }

        [Test]
        public void EffectTargetsRejectMissingNativeCapabilities()
        {
            var host = new GameObject("Missing Motion parameter host");
            try
            {
                ObjectId id = Id();
                ObjectId clock = Id();
                var target = new BattlementWorldMotionTarget(host.transform);
                BattlementUiException failure = Assert.Throws<BattlementUiException>(() =>
                    target.Configure(Descriptor(id, clock, Track(MotionProperty.LightIntensity, 1)))
                )!;
                Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.InvalidProperty));
            }
            finally
            {
                Object.DestroyImmediate(host);
            }
        }

        [Test]
        public void AudioWriterTargetsOnePlaybackOnTheSharedTimeline()
        {
            var host = new GameObject("Audio Motion host");
            try
            {
                ObjectId id = Id();
                ObjectId clock = Id();
                ObjectId playback = Id();
                ObjectId peer = Id();
                var audio = new AudioProbe((playback, 0.8f), (peer, 0.35f));
                MotionDescriptor descriptor = Descriptor(
                    id,
                    clock,
                    new MotionPropertyTrack(
                        MotionProperty.AudioVolume,
                        new MotionPropertyTarget.AudioVolume(playback),
                        new MotionValue[] { new MotionValue.Scalar(0.2) },
                        Tween()
                    )
                );
                var target = new BattlementWorldMotionTarget(host.transform, audio);
                target.Configure(descriptor);
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion.Prepare(target, id, descriptor)!.Commit();
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();

                Assert.That(audio.ReadMotionVolume(playback), Is.EqualTo(0.5).Within(0.00001));
                Assert.That(audio.ReadMotionVolume(peer), Is.EqualTo(0.35).Within(0.00001));
            }
            finally
            {
                Object.DestroyImmediate(host);
            }
        }

        [Test]
        public void MaterialWriterIsInstanceLocalAndUsesPreparedFloatType()
        {
            Material shared = null!;
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                shared = new Material(Shader.Find("UI/Default"));
                var address = new MaterialAddress("test/motion-material");
                ObjectId host = Id();
                ObjectId peer = Id();
                ObjectId clock = Id();
                harness.AssetStorage.EnqueueValue(shared);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.MaterialParameters(
                                address,
                                new[]
                                {
                                    new MaterialParameterDeclaration(
                                        "_StencilComp",
                                        MaterialParameterKind.Float
                                    ),
                                }
                            ),
                        },
                        objects: new[] { Card(host, address, 0.2), Card(peer, address, 0.4) }
                    )
                );
                harness.Runner.Connect();
                Renderer renderer = Identity(host).GetComponent<Renderer>();
                Renderer peerRenderer = Identity(peer).GetComponent<Renderer>();
                MotionDescriptor descriptor = Descriptor(
                    host,
                    clock,
                    new MotionPropertyTrack(
                        MotionProperty.MaterialScalar,
                        new MotionPropertyTarget.MaterialScalar(0, "_StencilComp"),
                        new MotionValue[] { new MotionValue.Scalar(0.8) },
                        Tween()
                    )
                );
                var target = new BattlementWorldMotionTarget(renderer.transform);
                target.Configure(descriptor);
                using var motion = new BattlementMotionWorld(registerPlayerLoop: false);
                motion.Prepare(target, host, descriptor)!.Commit();
                motion.SetControlledClock(clock, 500_000);
                motion.PostLayout();

                Assert.That(Scalar(renderer, "_StencilComp"), Is.EqualTo(0.5).Within(0.00001));
                Assert.That(Scalar(peerRenderer, "_StencilComp"), Is.EqualTo(0.4).Within(0.00001));
                Assert.That(renderer.sharedMaterial, Is.SameAs(shared));
                Assert.That(peerRenderer.sharedMaterial, Is.SameAs(shared));
            }
            finally
            {
                Object.DestroyImmediate(shared);
            }
        }

        private static MotionPropertyTrack Track(MotionProperty property, double value) =>
            new(
                property,
                new MotionPropertyTarget.Host(),
                new MotionValue[] { new MotionValue.Scalar(value) },
                Tween()
            );

        private static TransitionDefinition Tween() =>
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

        private static MotionDescriptor Descriptor(
            ObjectId host,
            ObjectId clock,
            params MotionPropertyTrack[] tracks
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
                        new MotionTargetDescriptor(tracks, Array.Empty<MotionPropertyValue>()),
                        new MotionCallbackSubscriptions(false, false, false, false, false, false)
                    ),
                },
                new MotionClockSource.Controlled(clock),
                ReducedMotionPolicy.Never
            );

        private static ObjectId Id() => new(Guid.NewGuid());

        private static BattlementGameObject Card(
            ObjectId id,
            MaterialAddress address,
            double value
        ) =>
            new(id, new GameObjectKind.Quad(new[] { new MaterialAssignment(0, address) }))
            {
                ParentScene = new ParentScene.Persistent(),
                MaterialInstances = new[]
                {
                    new MaterialInstance(
                        address,
                        0,
                        new[]
                        {
                            new MaterialParameterValue(
                                "_StencilComp",
                                MaterialParameterKind.Float,
                                value,
                                0,
                                0,
                                0
                            ),
                        }
                    ),
                },
            };

        private static BattlementIdentity Identity(ObjectId id) =>
            Object.FindObjectsByType<BattlementIdentity>().Single(value => value.Id == id.Value);

        private static float Scalar(Renderer renderer, string name)
        {
            var block = new MaterialPropertyBlock();
            renderer.GetPropertyBlock(block, 0);
            return block.GetFloat(name);
        }

        private sealed class AudioProbe : IBattlementMotionAudio
        {
            private readonly System.Collections.Generic.Dictionary<Guid, float> values = new();

            internal AudioProbe(params (ObjectId Id, float Volume)[] initial)
            {
                foreach ((ObjectId id, float volume) in initial)
                    values.Add(id.Value, volume);
            }

            public bool HasMotionPlayback(ObjectId playbackId) =>
                values.ContainsKey(playbackId.Value);

            public float ReadMotionVolume(ObjectId playbackId) => values[playbackId.Value];

            public void WriteMotionVolume(ObjectId playbackId, double value) =>
                values[playbackId.Value] = (float)value;
        }
    }
}
