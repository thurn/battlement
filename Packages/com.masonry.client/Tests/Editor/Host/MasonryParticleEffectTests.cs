#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;
using UVector3 = UnityEngine.Vector3;

namespace Masonry.Tests
{
    public sealed class MasonryParticleEffectTests
    {
        [SetUp]
        public void SetUp() => PoolResetRecorder.Events.Clear();

        [Test]
        public void PlayAndStopApplyRecursivelyWithoutInferringCompletion()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new PrefabAddress("game/particle-root");
            var objectId = new ObjectId(Guid.NewGuid());
            GameObject prefab = ParticlePrefab(address.Value);
            harness.AssetStorage.EnqueueValue(prefab);
            SessionId session = Connect(
                harness,
                new PreparedAsset[] { new PreparedAsset.Prefab(address) },
                new[] { PrefabObject(objectId, address) }
            );
            GameObject instance = Find(objectId);
            ParticleSystem[] systems = instance.GetComponentsInChildren<ParticleSystem>(true);

            Submit(
                harness,
                session,
                Command(new CommandBody.Particle.Play(objectId)).Nonblocking()
            );
            Assert.That(systems.All(system => system.isPlaying), Is.True);
            foreach (ParticleSystem system in systems)
            {
                system.Emit(3);
            }

            Submit(
                harness,
                session,
                Command(new CommandBody.Particle.Stop(objectId, Clear: false))
            );
            Assert.That(systems.All(system => !system.isEmitting), Is.True);
            Assert.That(systems.Sum(system => system.particleCount), Is.GreaterThan(0));

            Submit(
                harness,
                session,
                Command(new CommandBody.Particle.Play(objectId, Restart: true)).Nonblocking(),
                Command(new CommandBody.Particle.Stop(objectId, Clear: true))
            );
            Assert.That(systems.All(system => !system.isPlaying), Is.True);
            Assert.That(systems.Sum(system => system.particleCount), Is.Zero);

            Submit(
                harness,
                session,
                Command(new CommandBody.Particle.Play(objectId)),
                reportsFailure: true
            );
            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.InvalidProperty)
            );
        }

        [Test]
        public void BlockingSpawnUsesBothLocationsAndReleasesNonpooledInstances()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new ParticleEffectAddress("game/dust");
            var anchorId = new ObjectId(Guid.NewGuid());
            var afterId = new ObjectId(Guid.NewGuid());
            GameObject prefab = ParticlePrefab(address.Value);
            harness.AssetStorage.EnqueueValue(prefab);
            SessionId session = Connect(
                harness,
                new PreparedAsset[] { new PreparedAsset.ParticleEffect(address) },
                new[] { Empty(anchorId, new Vector3(4, 5, 6)) }
            );
            Command spawn = Command(
                new CommandBody.Particle.Spawn(
                    address,
                    new ParticleSpawnLocation.AtGameObject(anchorId),
                    TimeSpan.FromMilliseconds(100)
                )
            );

            SubmitGroups(
                harness,
                session,
                new[] { spawn },
                new[] { Command(new CommandBody.Object.Create(Empty(afterId))) }
            );
            GameObject active = Spawned(prefab).Single();
            Assert.That(active.transform.position, Is.EqualTo(new UVector3(4, 5, 6)));
            Assert.That(active.activeSelf, Is.True);
            Assert.That(HasIdentity(afterId), Is.False);

            Advance(harness, 100);
            Assert.That(active == null, Is.True);
            Assert.That(HasIdentity(afterId), Is.True);

            Command worldSpawn = Command(
                    new CommandBody.Particle.Spawn(
                        address,
                        new ParticleSpawnLocation.AtWorldPosition(new Vector3(-2, 3, 8)),
                        TimeSpan.FromSeconds(10)
                    )
                )
                .Nonblocking();
            Submit(harness, session, worldSpawn, Cancel(worldSpawn.Id));
            Assert.That(Spawned(prefab), Is.Empty);

            Submit(
                harness,
                session,
                Command(
                    new CommandBody.Particle.Spawn(
                        address,
                        new ParticleSpawnLocation.AtWorldPosition(Vector3.Zero),
                        TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1)
                    )
                ),
                reportsFailure: true
            );
            Assert.That(
                Failures(harness).Last().ErrorCode,
                Is.EqualTo(CoreErrorCode.LimitExceeded)
            );

            Submit(
                harness,
                session,
                Command(new CommandBody.Assets.ReplaceSet(FixtureAssets(harness)))
            );
            Assert.That(
                harness.Runner.TryGetPreparedAsset(
                    new PreparedAsset.ParticleEffect(address),
                    out _
                ),
                Is.False
            );
        }

        [Test]
        public void PoolReusesInComponentOrderEnforcesCapAndRetainsLeasesUntilCleared()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new ParticleEffectAddress("game/pooled");
            GameObject prefab = ParticlePrefab(address.Value);
            prefab.AddComponent<MasonryEffectPool>().MaxInactiveCount = 1;
            prefab.AddComponent<PoolResetRecorder>().Label = "first";
            prefab.AddComponent<PoolResetRecorder>().Label = "second";
            harness.AssetStorage.EnqueueValue(prefab);
            PreparedAsset effect = new PreparedAsset.ParticleEffect(address);
            SessionId session = Connect(
                harness,
                new[] { effect },
                Array.Empty<MasonryGameObject>()
            );
            Command first = Spawn(address, 100).Nonblocking();
            Command second = Spawn(address, 100).Nonblocking();

            Submit(harness, session, first, second);
            GameObject[] originalInstances = Spawned(prefab);
            Assert.That(originalInstances, Has.Length.EqualTo(2));
            Assert.That(
                PoolResetRecorder.Events.Select(value => value.Action),
                Is.EqualTo(
                    new[] { "acquire:first", "acquire:second", "acquire:first", "acquire:second" }
                )
            );

            Advance(harness, 100);
            GameObject inactive = Spawned(prefab).Single();
            Assert.That(inactive.activeSelf, Is.False);
            Assert.That(originalInstances, Does.Contain(inactive));
            Assert.That(inactive.transform.position, Is.EqualTo(UVector3.zero));
            Assert.That(inactive.transform.rotation, Is.EqualTo(UnityEngine.Quaternion.identity));
            Assert.That(inactive.transform.localScale, Is.EqualTo(UVector3.one));

            Submit(harness, session, Spawn(address, 100).Nonblocking());
            GameObject reused = Spawned(prefab).Single();
            Assert.That(reused, Is.SameAs(inactive));
            Advance(harness, 100);

            Submit(
                harness,
                session,
                Command(new CommandBody.Assets.ReplaceSet(FixtureAssets(harness))),
                reportsFailure: true
            );
            Assert.That(Failures(harness).Last().ErrorCode, Is.EqualTo(CoreErrorCode.AssetInUse));

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

            Assert.That(Spawned(prefab), Is.Empty);
            Assert.That(harness.Runner.TryGetPreparedAsset(effect, out _), Is.False);
        }

        [Test]
        public void ResetExceptionDestroysTheInstanceAndReportsTheOperationFailure()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new ParticleEffectAddress("game/broken-reset");
            GameObject prefab = ParticlePrefab(address.Value);
            prefab.AddComponent<MasonryEffectPool>().MaxInactiveCount = 2;
            prefab.AddComponent<ThrowingPoolReset>();
            harness.AssetStorage.EnqueueValue(prefab);
            SessionId session = Connect(
                harness,
                new PreparedAsset[] { new PreparedAsset.ParticleEffect(address) },
                Array.Empty<MasonryGameObject>()
            );
            Command spawn = Spawn(address, 10).Nonblocking();

            Submit(harness, session, spawn);
            harness.Transport.EnqueueSubmit(
                FakeMasonryTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                )
            );
            Advance(harness, 10);

            Assert.That(Spawned(prefab), Is.Empty);
            Assert.That(OperationFailures(harness).Single().CommandId, Is.EqualTo(spawn.Id));
            Assert.That(
                OperationFailures(harness).Single().ErrorCode,
                Is.EqualTo(CoreErrorCode.UnityException)
            );
        }

        private static GameObject ParticlePrefab(string name)
        {
            var root = new GameObject(name);
            Configure(root.AddComponent<ParticleSystem>());
            var child = new GameObject("Child particles");
            child.transform.SetParent(root.transform, false);
            Configure(child.AddComponent<ParticleSystem>());
            root.SetActive(false);
            return root;
        }

        private static void Configure(ParticleSystem system)
        {
            ParticleSystem.MainModule main = system.main;
            main.playOnAwake = false;
            main.loop = true;
        }

        private static SessionId Connect(
            MasonryTestHarness harness,
            IReadOnlyList<PreparedAsset> assets,
            IReadOnlyList<MasonryGameObject> objects
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    preparedAssets: assets,
                    objects: objects
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static MasonryGameObject PrefabObject(ObjectId id, PrefabAddress address) =>
            new(
                id,
                new GameObjectKind.Prefab(address, Array.Empty<MaterialAssignment>(), null),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static MasonryGameObject Empty(ObjectId id, Vector3? position = null) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(position ?? Vector3.Zero, Quaternion.Identity, Vector3.One),
                Array.Empty<PointerEvent>()
            );

        private static Command Spawn(ParticleEffectAddress address, double lifetimeMs) =>
            Command(
                new CommandBody.Particle.Spawn(
                    address,
                    new ParticleSpawnLocation.AtWorldPosition(new Vector3(2, 4, 6)),
                    TimeSpan.FromMilliseconds(lifetimeMs)
                )
            );

        private static Command Command(CommandBody body) =>
            new(new CommandId(Guid.NewGuid()), body);

        private static Command Cancel(CommandId id) =>
            Command(new CommandBody.Operation.Cancel(id));

        private static PreparedAsset[] FixtureAssets(MasonryTestHarness harness) =>
            harness.AssetStorage.PrepareCalls.Where(FakeMasonryTransport.IsFixtureAsset).ToArray();

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
            if (second != null)
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

        private static GameObject Find(ObjectId id) =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Single(value => value.Id == id.Value)
                .gameObject;

        private static bool HasIdentity(ObjectId id) =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Any(value => value.Id == id.Value);

        private static GameObject[] Spawned(GameObject prefab) =>
            Object
                .FindObjectsByType<ParticleSystem>(FindObjectsInactive.Include)
                .Select(value => value.transform.root.gameObject)
                .Where(value =>
                    value != prefab && value.name.StartsWith(prefab.name, StringComparison.Ordinal)
                )
                .Distinct()
                .ToArray();

        private static BatchFailed<CoreErrorCode>[] Failures(MasonryTestHarness harness) =>
            Messages(harness)
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(value => value.Failure)
                .ToArray();

        private static OperationFailed<CoreErrorCode>[] OperationFailures(
            MasonryTestHarness harness
        ) =>
            Messages(harness)
                .OfType<ClientMessage<CoreErrorCode, byte>.OperationFailedMessage>()
                .Select(value => value.Failure)
                .ToArray();

        private static IEnumerable<ClientMessage<CoreErrorCode, byte>> Messages(
            MasonryTestHarness harness
        ) =>
            harness
                .Transport.SubmitMessages.Where(bytes => bytes.Length > 1)
                .Select(bytes =>
                    MasonryMessagePack.DeserializeClientMessage(
                        bytes,
                        new CoreErrorFormatter(),
                        new UnusedPayloadFormatter()
                    )
                );

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

    public sealed class PoolResetRecorder : MonoBehaviour, IMasonryPoolReset
    {
        public static List<(string Action, GameObject Instance)> Events { get; } = new();

        public string Label = string.Empty;

        public void OnMasonryAcquire() => Events.Add(($"acquire:{Label}", gameObject));

        public void OnMasonryRelease() => Events.Add(($"release:{Label}", gameObject));
    }

    public sealed class ThrowingPoolReset : MonoBehaviour, IMasonryPoolReset
    {
        public void OnMasonryAcquire() { }

        public void OnMasonryRelease() =>
            throw new InvalidOperationException("release reset failed");
    }
}
