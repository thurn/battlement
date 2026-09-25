#nullable enable

using System;
using System.Collections;
using System.Linq;
using Battlement.UI;
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

        [Test]
        public void ControlledAudioGraphsIgnoreDeviceTimeAndHonorPlaybackControls()
        {
            using var fixture = new Fixture();
            fixture.BeginControlled();
            var audio = fixture.Play(AudioBus.Music, 1, pitch: 2);
            fixture.BindAudioTime(audio.Command.Id, audio.Source);
            fixture.Step(15);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
            audio.Source.timeSamples = 8000;
            fixture.Advance(90000);
            fixture.Step(1, advance: false);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
            fixture.Submit(new CommandBody.Audio.Pause(audio.Command.Id));
            fixture.Step(30);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(1).Within(0.00001));
            fixture.Submit(new CommandBody.Audio.Resume(audio.Command.Id));
            fixture.Step(15);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
            fixture.Submit(new CommandBody.Audio.SetBuffering(audio.Command.Id, true));
            fixture.Step(30);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(2).Within(0.00001));
            fixture.Submit(new CommandBody.Audio.SetBuffering(audio.Command.Id, false));
            fixture.Submit(new CommandBody.Audio.Seek(audio.Command.Id, TimeSpan.FromSeconds(9.5)));
            fixture.Step(15);
            Assert.That(audio.Source.transform.localPosition.x, Is.EqualTo(0.5).Within(0.00001));
            fixture.Submit(new CommandBody.Audio.Replace(audio.Command.Id, Fixture.Replacement));
            fixture.Step(1, advance: false);
            Assert.That(audio.Source.transform.localPosition.x, Is.Zero);
        }

        [Test]
        public void ControlledFiniteAudioCompletesAtLogicalDurationAfterPause()
        {
            using var fixture = new Fixture();
            fixture.BeginControlled();
            var audio = fixture.Play(AudioBus.Effects, 1, pitch: 2, loop: false);
            fixture.Step(30);
            fixture.Submit(new CommandBody.Audio.Pause(audio.Command.Id));
            fixture.Step(300);
            Assert.That(audio.Source.gameObject.activeSelf, Is.True);
            fixture.Submit(new CommandBody.Audio.Resume(audio.Command.Id));
            fixture.Step(119);
            Assert.That(audio.Source.gameObject.activeSelf, Is.True);
            fixture.Step(1);
            Assert.That(audio.Source.gameObject.activeSelf, Is.False);
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
            private DittoMotionController? motion;
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

            internal void BeginControlled()
            {
                motion = new DittoMotionController(harness.Runner);
                motion.Begin(DittoMotion.Controlled);
            }

            internal void BindAudioTime(CommandId playback, AudioSource source)
            {
                var host = new ObjectId(Guid.NewGuid());
                var value = new ObjectId(Guid.NewGuid());
                MotionDescriptor descriptor = SharedMotionDriverTests.Descriptor(
                    host,
                    host,
                    MotionProperty.LocalPositionX,
                    1
                ) with
                {
                    Slots = Array.Empty<MotionSlotDescriptor>(),
                    Values = new[]
                    {
                        new MotionValueDescriptor(
                            value,
                            new MotionValue.Scalar(0),
                            new MotionValueSource.Time(
                                new MotionClockSource.Audio(new ObjectId(playback.Value))
                            )
                        ),
                    },
                    ValueBindings = new[]
                    {
                        new MotionValueBinding(MotionProperty.LocalPositionX, value),
                    },
                };
                harness
                    .Runner.UiDocumentsForTests.MotionWorldForTests.Prepare(
                        new BattlementWorldMotionTarget(source.transform),
                        host,
                        descriptor
                    )!
                    .Commit();
            }

            internal void Step(int frames, bool advance = true)
            {
                for (int index = 0; index < frames; index++)
                {
                    motion!.PrepareFrame(forceAdvance: advance);
                    harness.Runner.RunFrame();
                    BattlementMotionWorld world = harness
                        .Runner
                        .UiDocumentsForTests
                        .MotionWorldForTests;
                    world.PreLayout();
                    world.PostLayout();
                    harness.Runner.CompleteNativeFrame();
                    motion.ObserveCommittedFrame();
                }
            }

            internal void Mix(AudioMix mix) => Submit(new CommandBody.Audio.SetMix(mix));

            internal (Command Command, AudioSource Source) Play(
                AudioBus bus,
                double volume,
                double fadeMilliseconds = 0,
                double pitch = 1,
                bool loop = true
            )
            {
                AudioSource[] existing = ActiveSources();
                Command command = Submit(
                    new CommandBody.Audio.Play(
                        Address,
                        volume,
                        Pitch: pitch,
                        Loop: loop,
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
