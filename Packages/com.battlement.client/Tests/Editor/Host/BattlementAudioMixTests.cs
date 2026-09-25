#nullable enable

using System;
using System.Collections;
using System.Linq;
using NUnit.Framework;
using Unity.Collections;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.TestTools;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementAudioMixTests
    {
        [Test]
        public void LiveMixerChangesComposeWithCrossfadesAndMuteWithoutRestartingSounds()
        {
            using var fixture = new Fixture();
            fixture.Mix(new AudioMix(0.5, 0.4, 0.2));
            var music = fixture.Play(AudioBus.Music, 0.8, 1000);
            var effect = fixture.Play(AudioBus.Effects, 0.6);
            fixture.Advance(250);
            Assert.That(music.Source.volume, Is.EqualTo(0.04f).Within(0.001f));
            Assert.That(effect.Source.volume, Is.EqualTo(0.06f).Within(0.001f));

            fixture.Submit(new CommandBody.Audio.Stop(music.Command.Id, TimeSpan.FromSeconds(1)));
            var incoming = fixture.Play(AudioBus.Music, 0.8, 1000);
            fixture.Advance(250);
            fixture.Mix(new AudioMix(1, 0.5, 0.4));
            Assert.That(music.Source.volume, Is.EqualTo(0.075f).Within(0.001f));
            Assert.That(incoming.Source.volume, Is.EqualTo(0.1f).Within(0.001f));
            Assert.That(effect.Source.volume, Is.EqualTo(0.24f).Within(0.001f));

            incoming.Source.timeSamples = 500;
            int playhead = incoming.Source.timeSamples;
            AudioClip clip = incoming.Source.clip;
            fixture.Mix(new AudioMix(1, 0.5, 0.4, muted: true));
            Assert.That(incoming.Source.timeSamples, Is.EqualTo(playhead));
            Assert.That(incoming.Source.clip, Is.SameAs(clip));
            Assert.That(Fixture.ActiveSources().All(source => source.volume == 0), Is.True);
            fixture.Advance(250);
            fixture.Mix(new AudioMix(1, 0.5, 0.4));
            Assert.That(music.Source.volume, Is.EqualTo(0.05f).Within(0.001f));
            Assert.That(incoming.Source.volume, Is.EqualTo(0.2f).Within(0.001f));
            fixture.Advance(500);
            Assert.That(music.Source.gameObject.activeSelf, Is.False);
            Assert.That(incoming.Source.volume, Is.EqualTo(0.4f).Within(0.001f));
        }

        [Test]
        public void ReusedSourcesAndReplacementClipsKeepTheCurrentBusAndFade()
        {
            using var fixture = new Fixture();
            fixture.Mix(new AudioMix(0.5, 0.2, 0.8));
            var music = fixture.Play(AudioBus.Music, 1);
            Assert.That(music.Source.volume, Is.EqualTo(0.1f).Within(0.001f));
            fixture.Submit(new CommandBody.Audio.Stop(music.Command.Id));
            var effect = fixture.Play(AudioBus.Effects, 0.5, 1000);
            Assert.That(effect.Source, Is.SameAs(music.Source));
            fixture.Advance(400);
            Assert.That(effect.Source.volume, Is.EqualTo(0.08f).Within(0.001f));
            fixture.Submit(new CommandBody.Audio.Replace(effect.Command.Id, Fixture.Replacement));
            Assert.That(effect.Source.clip, Is.SameAs(fixture.ReplacementClip));
            fixture.Advance(200);
            Assert.That(effect.Source.volume, Is.EqualTo(0.12f).Within(0.001f));
            fixture.Mix(new AudioMix(0, 1, 1));
            Assert.That(effect.Source.volume, Is.Zero);
            fixture.Mix(new AudioMix(1, 0, 0.5));
            Assert.That(effect.Source.volume, Is.EqualTo(0.15f).Within(0.001f));
        }

        [UnityTest]
        [Explicit("Native DSP output probe; run by exact test name with an audio-capable editor.")]
        [Timeout(60000)]
        public IEnumerator NativeOutputTracksLiveGainsAndMuteDuringCrossfade()
        {
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            yield return new EnterPlayMode();
            var settle = new WaitForSecondsRealtime(0.3f);
            using (var fixture = new Fixture(tone: true))
            {
                yield return fixture.WaitUntilReady();
                using var recording = new OutputRecording();
                var music = fixture.Play(AudioBus.Music, 0.8);
                yield return settle;
                Debug.Log(
                    $"Audio probe source: playing={music.Source.isPlaying}, "
                        + $"volume={music.Source.volume}, samples={music.Source.timeSamples}, "
                        + $"listenerPause={AudioListener.pause}, "
                        + $"listenerVolume={AudioListener.volume}, "
                        + $"outputRate={AudioSettings.outputSampleRate}"
                );
                double baseline = OutputRms();
                Assert.That(baseline, Is.GreaterThan(0.001), "Native DSP output is unavailable.");
                int playhead = music.Source.timeSamples;
                fixture.Mix(new AudioMix(0.5, 0.4, 1));
                yield return settle;
                double reduced = OutputRms();
                Assert.That(reduced / baseline, Is.EqualTo(0.2).Within(0.03));
                Assert.That(music.Source.timeSamples, Is.GreaterThan(playhead));
                fixture.Mix(new AudioMix(1, 0, 1));
                yield return settle;
                Assert.That(OutputRms(), Is.LessThan(0.00001));
                var effect = fixture.Play(AudioBus.Effects, 0.8);
                yield return settle;
                double effectBaseline = OutputRms();
                fixture.Mix(new AudioMix(0.5, 0, 0.6));
                yield return settle;
                double effectReduced = OutputRms();
                Assert.That(effectReduced / effectBaseline, Is.EqualTo(0.3).Within(0.04));
                fixture.Submit(new CommandBody.Audio.Stop(effect.Command.Id));
                fixture.Mix(AudioMix.FullVolume);
                fixture.Submit(
                    new CommandBody.Audio.Stop(music.Command.Id, TimeSpan.FromSeconds(1))
                );
                var incoming = fixture.Play(AudioBus.Music, 0.8, 1000);
                fixture.Advance(250);
                Assert.That(music.Source.volume, Is.EqualTo(0.6f).Within(0.001f));
                Assert.That(incoming.Source.volume, Is.EqualTo(0.2f).Within(0.001f));
                playhead = incoming.Source.timeSamples;
                fixture.Mix(new AudioMix(1, 1, 1, muted: true));
                yield return settle;
                double muted = OutputRms();
                Assert.That(muted, Is.LessThan(0.00001));
                Assert.That(incoming.Source.timeSamples, Is.GreaterThan(playhead));
                fixture.Advance(250);
                fixture.Mix(new AudioMix(0.5, 0.5, 1));
                Assert.That(music.Source.volume, Is.EqualTo(0.1f).Within(0.001f));
                Assert.That(incoming.Source.volume, Is.EqualTo(0.1f).Within(0.001f));
                fixture.Advance(500);
                yield return settle;
                double resumed = OutputRms();
                Assert.That(resumed / baseline, Is.EqualTo(0.25).Within(0.04));
                Assert.That(music.Source.gameObject.activeSelf, Is.False);
                Debug.Log(
                    $"Audio output probe: music RMS {baseline:R} -> {reduced:R}; "
                        + $"effects {effectBaseline:R} -> {effectReduced:R}; "
                        + $"muted {muted:R}; resumed {resumed:R}; "
                        + "playhead advanced through mix/mute; crossfade completed."
                );
            }
            yield return new ExitPlayMode();
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        }

        private static double OutputRms()
        {
            using var samples = new NativeArray<float>(8192, Allocator.Temp);
            for (int buffer = 0; buffer < 3; buffer++)
                Assert.That(AudioRenderer.Render(samples), Is.True);
            return Math.Sqrt(samples.ToArray().Average(value => (double)value * value));
        }

        private sealed class OutputRecording : IDisposable
        {
            internal OutputRecording() => Assert.That(AudioRenderer.Start(), Is.True);

            public void Dispose() => AudioRenderer.Stop();
        }

        private sealed class Fixture : IDisposable
        {
            internal static readonly AudioClipAddress Address = new("audio/mix");
            internal static readonly AudioClipAddress Replacement = new("audio/replacement");
            private readonly BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            private readonly SessionId session = new(Guid.NewGuid());
            private readonly AudioClip clip = AudioClip.Create("mix", 10000, 1, 1000, false);

            internal AudioClip ReplacementClip { get; } =
                AudioClip.Create("replacement", 10000, 1, 1000, false);

            internal Fixture(bool tone = false)
            {
                if (tone)
                {
                    var samples = new float[clip.samples];
                    for (int index = 0; index < samples.Length; index++)
                        samples[index] =
                            0.1f * Mathf.Sin(2 * Mathf.PI * 100 * index / clip.frequency);
                    clip.SetData(samples, 0);
                }
                harness.AssetStorage.EnqueueValue(clip);
                harness.AssetStorage.EnqueueValue(ReplacementClip);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        session,
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.AudioClip(Address),
                            new PreparedAsset.AudioClip(Replacement),
                        }
                    )
                );
                harness.Runner.Connect();
                Assert.That(
                    harness.Runner.CurrentFailure,
                    Is.Null,
                    string.Join("\n", harness.Logger.Records.Select(record => record.Message))
                );
            }

            internal IEnumerator WaitUntilReady()
            {
                float deadline = Time.realtimeSinceStartup + 5;
                while (!harness.Runner.IsInputAvailable)
                {
                    Assert.That(harness.Runner.CurrentFailure, Is.Null);
                    Assert.That(
                        Time.realtimeSinceStartup,
                        Is.LessThan(deadline),
                        string.Join("\n", harness.Logger.Records.Select(record => record.Message))
                    );
                    yield return null;
                }
            }

            internal void Mix(AudioMix mix) => Submit(new CommandBody.Audio.SetMix(mix));

            internal (Command Command, AudioSource Source) Play(
                AudioBus bus,
                double volume,
                double fadeMilliseconds = 0
            )
            {
                AudioSource[] existing = ActiveSources();
                Command command = Submit(
                    new CommandBody.Audio.Play(
                        Address,
                        volume,
                        Loop: true,
                        FadeIn: TimeSpan.FromMilliseconds(fadeMilliseconds),
                        Bus: bus
                    )
                );
                return (command, ActiveSources().Single(source => !existing.Contains(source)));
            }

            internal Command Submit(CommandBody body)
            {
                Command command = new Command(new CommandId(Guid.NewGuid()), body).Nonblocking();
                var batch = new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    new[] { new ParallelCommandGroup<Command>(new[] { command }) }
                );
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(
                            session,
                            new ResponseMessage<Command>[]
                            {
                                new ResponseMessage<Command>.BatchMessage(batch),
                            }
                        )
                    )
                );
                harness.Runner.Submit(new byte[] { 1 });
                Assert.That(harness.Transport.BatchFailures, Is.Empty);
                return command;
            }

            internal void Advance(double milliseconds)
            {
                harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
                harness.Runner.RunFrame();
            }

            internal static AudioSource[] ActiveSources() =>
                Object
                    .FindObjectsByType<AudioSource>()
                    .Where(source => source.gameObject.name == "Battlement Audio Source")
                    .ToArray();

            public void Dispose()
            {
                harness.Dispose();
                Object.DestroyImmediate(clip);
                Object.DestroyImmediate(ReplacementClip);
            }
        }
    }
}
