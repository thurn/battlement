#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryAudioCommandTests
    {
        [Test]
        public void PlayPlacesAConfiguredSourceAtTheCurrentInputCamera()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create(useInstantAnimations: false);
            AudioClip clip = Clip(0.2f);
            try
            {
                var address = new AudioClipAddress("game/audio/camera");
                var firstCamera = new ObjectId(Guid.NewGuid());
                var secondCamera = new ObjectId(Guid.NewGuid());
                harness.AssetStorage.EnqueueValue(clip);
                SessionId session = Connect(
                    harness,
                    address,
                    new[] { Camera(firstCamera), Camera(secondCamera) },
                    firstCamera
                );
                Command play = Command(
                        new CommandBody.Audio.Play(
                            address,
                            Volume: 0.8,
                            Pitch: 2,
                            Loop: true,
                            FadeIn: TimeSpan.FromMilliseconds(100)
                        )
                    )
                    .Nonblocking();

                Submit(harness, session, play);
                AudioSource source = Sources().Single();
                Assert.That(Object.FindObjectsByType<AudioListener>(), Has.Length.EqualTo(1));
                Assert.That(source.transform.parent, Is.SameAs(Find(firstCamera).transform));
                Assert.That(source.transform.localPosition, Is.EqualTo(UnityEngine.Vector3.zero));
                Assert.That(source.spatialBlend, Is.Zero);
                Assert.That(source.pitch, Is.EqualTo(2f));
                Assert.That(source.loop, Is.True);
                Assert.That(source.clip, Is.SameAs(clip));
                Assert.That(source.volume, Is.Zero);

                Advance(harness, 50);
                Assert.That(source.volume, Is.EqualTo(0.4f).Within(0.001f));
                Submit(harness, session, Command(new CommandBody.Input.SetCamera(secondCamera)));
                Assert.That(source.transform.parent, Is.SameAs(Find(secondCamera).transform));
                Assert.That(source.clip, Is.SameAs(clip));
                Assert.That(source.timeSamples, Is.GreaterThanOrEqualTo(0));
            }
            finally
            {
                harness.Dispose();
                Object.DestroyImmediate(clip);
            }
        }

        [Test]
        public void FiniteBlockingPlayCompletesByClipDurationAndReusesTheSource()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create();
            AudioClip clip = Clip(0.2f);
            try
            {
                var address = new AudioClipAddress("game/audio/finite");
                var createdId = new ObjectId(Guid.NewGuid());
                harness.AssetStorage.EnqueueValue(clip);
                SessionId session = Connect(harness, address);
                Command play = Command(new CommandBody.Audio.Play(address, Pitch: 2));

                SubmitGroups(
                    harness,
                    session,
                    new[] { play },
                    new[] { Command(new CommandBody.Object.Create(Empty(createdId))) }
                );
                Assert.That(
                    Failures(harness).Select(value => $"{value.ErrorCode}: {value.Message}"),
                    Is.Empty
                );
                AudioSource source = Sources().Single();
                Assert.That(source.gameObject.activeSelf, Is.True);
                Assert.That(HasIdentity(createdId), Is.False);

                Advance(harness, 99);
                Assert.That(HasIdentity(createdId), Is.False);
                Advance(harness, 1);
                Assert.That(HasIdentity(createdId), Is.True);
                Assert.That(source.gameObject.activeSelf, Is.False);
                Assert.That(source.clip, Is.Null);

                Command loop = Command(new CommandBody.Audio.Play(address, Loop: true))
                    .Nonblocking();
                Submit(harness, session, loop);
                Assert.That(Sources().Single(), Is.SameAs(source));
                Assert.That(source.gameObject.activeSelf, Is.True);
                Submit(harness, session, Command(new CommandBody.Audio.Stop(loop.Id)));
                Assert.That(source.gameObject.activeSelf, Is.False);
            }
            finally
            {
                harness.Dispose();
                Object.DestroyImmediate(clip);
            }
        }

        [Test]
        public void VolumeWritesTweenWithConflictsAndStopFadeBlocksItsGroup()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create(useInstantAnimations: false);
            AudioClip clip = Clip(1f);
            try
            {
                var address = new AudioClipAddress("game/audio/volume");
                var createdId = new ObjectId(Guid.NewGuid());
                harness.AssetStorage.EnqueueValue(clip);
                SessionId session = Connect(harness, address);
                Command play = Command(new CommandBody.Audio.Play(address, Volume: 0.8, Loop: true))
                    .Nonblocking();
                Submit(harness, session, play);
                AudioSource source = Sources().Single();

                Submit(
                    harness,
                    session,
                    Command(new CommandBody.Audio.SetVolume(play.Id, 0.6)),
                    Command(
                            new CommandBody.Audio.TweenVolume(
                                play.Id,
                                0.2,
                                new Tween(
                                    TimeSpan.FromMilliseconds(100),
                                    TimeSpan.Zero,
                                    Easing.Linear,
                                    new TweenRepeat.Once()
                                )
                            )
                        )
                        .Nonblocking()
                );
                Advance(harness, 50);
                Assert.That(source.volume, Is.EqualTo(0.4f).Within(0.001f));

                Submit(harness, session, Command(new CommandBody.Audio.SetVolume(play.Id, 0.9)));
                Advance(harness, 100);
                Assert.That(source.volume, Is.EqualTo(0.9f).Within(0.001f));

                SubmitGroups(
                    harness,
                    session,
                    new[]
                    {
                        Command(
                            new CommandBody.Audio.Stop(play.Id, TimeSpan.FromMilliseconds(100))
                        ),
                    },
                    new[] { Command(new CommandBody.Object.Create(Empty(createdId))) }
                );
                Advance(harness, 50);
                Assert.That(source.volume, Is.EqualTo(0.45f).Within(0.001f));
                Assert.That(HasIdentity(createdId), Is.False);
                Advance(harness, 50);
                Assert.That(source.gameObject.activeSelf, Is.False);
                Assert.That(HasIdentity(createdId), Is.True);
            }
            finally
            {
                harness.Dispose();
                Object.DestroyImmediate(clip);
            }
        }

        [Test]
        public void ValidationLeasesAndSnapshotCancellationAreExternallyVisible()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create();
            AudioClip clip = Clip(1f);
            try
            {
                var address = new AudioClipAddress("game/audio/cancel");
                PreparedAsset asset = new PreparedAsset.AudioClip(address);
                harness.AssetStorage.EnqueueValue(clip);
                SessionId session = Connect(harness, address);

                Submit(
                    harness,
                    session,
                    Command(new CommandBody.Audio.Play(address, Volume: -0.1)),
                    reportsFailure: true
                );
                Submit(
                    harness,
                    session,
                    Command(new CommandBody.Audio.Play(address, Pitch: 0)),
                    reportsFailure: true
                );
                Submit(
                    harness,
                    session,
                    Command(new CommandBody.Audio.Play(address, Loop: true)),
                    reportsFailure: true
                );
                Submit(
                    harness,
                    session,
                    Command(
                        new CommandBody.Audio.Play(
                            address,
                            FadeIn: TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1)
                        )
                    ),
                    reportsFailure: true
                );
                Assert.That(
                    Failures(harness).TakeLast(4).Select(value => value.ErrorCode),
                    Is.EqualTo(
                        new[]
                        {
                            CoreErrorCode.InvalidProperty,
                            CoreErrorCode.InvalidProperty,
                            CoreErrorCode.InvalidProperty,
                            CoreErrorCode.LimitExceeded,
                        }
                    )
                );

                Command live = Command(new CommandBody.Audio.Play(address, Loop: true))
                    .Nonblocking();
                Submit(harness, session, live);
                AudioSource source = Sources().Single();
                Submit(
                    harness,
                    session,
                    Command(
                        new CommandBody.Audio.Stop(
                            live.Id,
                            TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1)
                        )
                    ),
                    reportsFailure: true
                );
                Assert.That(
                    Failures(harness).Last().ErrorCode,
                    Is.EqualTo(CoreErrorCode.LimitExceeded)
                );
                Assert.That(source.gameObject.activeSelf, Is.True);
                Submit(
                    harness,
                    session,
                    Command(new CommandBody.Assets.ReplaceSet(FixtureAssets(harness))),
                    reportsFailure: true
                );
                Assert.That(
                    Failures(harness).Last().ErrorCode,
                    Is.EqualTo(CoreErrorCode.AssetInUse)
                );

                var snapshot = FakeMasonryTransport.CompleteSnapshot(session);
                var response = new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    }
                );
                harness.Transport.EnqueueSubmit(FakeMasonryTransport.ResponseResult(response));
                harness.Runner.Submit(new byte[] { 2 });

                Assert.That(source.gameObject.activeSelf, Is.False);
                Assert.That(source.clip, Is.Null);
                Assert.That(harness.Runner.TryGetPreparedAsset(asset, out _), Is.False);
                Advance(harness, 2_000);
                Assert.That(source.gameObject.activeSelf, Is.False);
            }
            finally
            {
                harness.Dispose();
                Object.DestroyImmediate(clip);
            }
        }

        private static SessionId Connect(
            MasonryTestHarness harness,
            AudioClipAddress address,
            IReadOnlyList<MasonryGameObject>? objects = null,
            ObjectId? inputCamera = null
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.AudioClip(address) },
                    objects: objects,
                    inputCameraId: inputCamera
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static AudioClip Clip(float seconds) =>
            AudioClip.Create("Masonry audio test", (int)(seconds * 1_000), 1, 1_000, false);

        private static MasonryGameObject Camera(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Camera(new CameraState()),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static MasonryGameObject Empty(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static Command Command(CommandBody body) =>
            new(new CommandId(Guid.NewGuid()), body);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            params Command[] commands
        ) => Submit(harness, session, commands, reportsFailure: false);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            Command command,
            bool reportsFailure
        ) => Submit(harness, session, new[] { command }, reportsFailure);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            Command[] commands,
            bool reportsFailure
        ) => SubmitGroups(harness, session, commands, reportsFailure: reportsFailure);

        private static void SubmitGroups(
            MasonryTestHarness harness,
            SessionId session,
            Command[] first,
            Command[]? second = null,
            bool reportsFailure = false
        )
        {
            var groups = new List<ParallelCommandGroup<Command>> { new(first) };
            if (second is not null)
            {
                groups.Add(new ParallelCommandGroup<Command>(second));
            }

            var batch = new Batch(new BatchId(Guid.NewGuid()), session, groups);
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeMasonryTransport.ResponseResult(response));
            if (reportsFailure)
            {
                harness.Transport.EnqueueSubmit(
                    FakeMasonryTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void Advance(MasonryTestHarness harness, double milliseconds)
        {
            harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
            harness.Runner.RunFrame();
        }

        private static AudioSource[] Sources() =>
            Object
                .FindObjectsByType<AudioSource>(FindObjectsInactive.Include)
                .Where(value => value.gameObject.name == "Masonry Audio Source")
                .ToArray();

        private static GameObject Find(ObjectId id) =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Single(value => value.Id == id.Value)
                .gameObject;

        private static bool HasIdentity(ObjectId id) =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Any(value => value.Id == id.Value);

        private static PreparedAsset[] FixtureAssets(MasonryTestHarness harness) =>
            harness.AssetStorage.PrepareCalls.Where(FakeMasonryTransport.IsFixtureAsset).ToArray();

        private static BatchFailed<CoreErrorCode>[] Failures(MasonryTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Where(bytes => bytes.Length > 1)
                .Select(bytes =>
                    MasonryMessagePack.DeserializeClientMessage(
                        bytes,
                        new CoreErrorFormatter(),
                        new UnusedPayloadFormatter()
                    )
                )
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(value => value.Failure)
                .ToArray();

        private sealed class CoreErrorFormatter : IMessagePackFormatter<CoreErrorCode>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                CoreErrorCode value,
                MessagePackSerializerOptions options
            ) => writer.Write(value.ToString());

            public CoreErrorCode Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => Enum.Parse<CoreErrorCode>(reader.ReadString()!);
        }

        private sealed class UnusedPayloadFormatter : IMessagePackFormatter<byte>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                byte value,
                MessagePackSerializerOptions options
            ) => writer.Write(value);

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => reader.ReadByte();
        }
    }
}
