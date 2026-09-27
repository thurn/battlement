#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;
using UVector3 = UnityEngine.Vector3;

namespace Battlement.Tests
{
    public sealed class BattlementParticleEffectTests
    {
        [SetUp]
        public void SetUp() => PoolResetRecorder.Events.Clear();

        [Test]
        public void PlayAndStopApplyRecursivelyWithoutInferringCompletion()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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
            Assert.That(Failures(harness), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(
                harness.Logger.Records.Last().Message,
                Does.Contain("Deferred response failed")
            );
        }

        [TestCase(false)]
        [TestCase(true)]
        public void DittoPrefabParticlesUseControlledTimeOrCanonicalInstantPhase(bool controlled)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(controlled ? DittoMotion.Controlled : DittoMotion.Instant);
            var address = new PrefabAddress("game/prewarmed-cursor");
            var objectId = new ObjectId(Guid.NewGuid());
            GameObject prefab = ParticlePrefab(address.Value);
            foreach (ParticleSystem system in prefab.GetComponentsInChildren<ParticleSystem>(true))
            {
                ParticleSystem.MainModule main = system.main;
                main.playOnAwake = true;
                main.prewarm = true;
                main.startLifetime = 2;
                main.startSpeed = 1;
            }
            harness.AssetStorage.EnqueueValue(prefab);
            SessionId session = Connect(
                harness,
                new PreparedAsset[] { new PreparedAsset.Prefab(address) },
                new[] { PrefabObject(objectId, address) }
            );
            ParticleSystem[] systems = Find(objectId).GetComponentsInChildren<ParticleSystem>(true);
            DittoCommittedFrame Frame(bool advance = false)
            {
                motion.PrepareFrame(forceAdvance: advance);
                harness.Runner.RunFrame();
                harness.Runner.CompleteNativeFrame();
                return motion.ObserveCommittedFrame();
            }
            _ = Frame();
            Assert.That(
                systems.All(system => system.particleCount > 0),
                Is.True,
                string.Join(
                    "; ",
                    systems.Select(system =>
                        $"{system.name}: active={system.gameObject.activeInHierarchy}, "
                        + $"time={system.time}, count={system.particleCount}, "
                        + $"awake={system.main.playOnAwake}, seed={system.randomSeed}"
                    )
                ) + $"; {motion.PendingDiagnostic()}"
            );
            float[] initialTimes = systems.Select(system => system.time).ToArray();
            _ = Frame();
            DittoCommittedFrame settled = Frame();
            Assert.That(settled.IsSettled, Is.True);
            Assert.That(settled.HasInfiniteOperations, Is.True);
            Assert.That(systems.Select(system => system.time), Is.EqualTo(initialTimes));
            Assert.That(systems.All(system => system.isPaused), Is.True);
            Assert.That(systems.All(system => !system.useAutoRandomSeed), Is.True);
            _ = Frame(advance: true);
            Assert.That(
                systems[0].time,
                controlled ? Is.Not.EqualTo(initialTimes[0]) : Is.EqualTo(initialTimes[0])
            );

            Find(objectId).SetActive(false);
            _ = Frame();
            Assert.That(systems.Sum(system => system.particleCount), Is.Zero);
            Find(objectId).SetActive(true);
            _ = Frame();
            Assert.That(systems.Select(system => system.time), Is.EqualTo(initialTimes));
            Assert.That(systems.All(system => system.particleCount > 0), Is.True);

            Submit(harness, session, Command(new CommandBody.Particle.Stop(objectId, Clear: true)));
            _ = Frame(advance: true);
            Assert.That(systems.Sum(system => system.particleCount), Is.Zero);
            Assert.That(harness.Runner.ObserveDittoWork().HasInfiniteOperations, Is.False);
        }

        [TestCase(false)]
        [TestCase(true)]
        public void DittoParticlesReproduceRestartAndExpireSpawn(bool controlled)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var motion = new DittoMotionController(harness.Runner);
            motion.Begin(controlled ? DittoMotion.Controlled : DittoMotion.Instant);
            var address = new PrefabAddress("game/seeded-root");
            var effectAddress = new ParticleEffectAddress("game/seeded-burst");
            var objectId = new ObjectId(Guid.NewGuid());
            GameObject prefab = ParticlePrefab(address.Value);
            GameObject burst = ParticlePrefab(effectAddress.Value);
            burst.AddComponent<BattlementEffectPool>().MaxInactiveCount = 1;
            harness.AssetStorage.EnqueueValue(prefab);
            harness.AssetStorage.EnqueueValue(burst);
            SessionId session = Connect(
                harness,
                new PreparedAsset[]
                {
                    new PreparedAsset.Prefab(address),
                    new PreparedAsset.ParticleEffect(effectAddress),
                },
                new[] { PrefabObject(objectId, address) }
            );
            ParticleSystem[] systems = Find(objectId).GetComponentsInChildren<ParticleSystem>(true);
            void Frame(bool advance = true)
            {
                motion.PrepareFrame(forceAdvance: advance, preserveTime: !advance);
                harness.Runner.RunFrame();
                harness.Runner.CompleteNativeFrame();
            }
            Command Play(uint seed) =>
                Command(new CommandBody.Particle.Play(objectId, Restart: true, Seed: seed))
                    .Nonblocking();
            Command Burst() =>
                Command(
                        new CommandBody.Particle.Spawn(
                            effectAddress,
                            new ParticleSpawnLocation.AtWorldPosition(Vector3.Zero),
                            TimeSpan.FromSeconds(1),
                            Seed: 81
                        )
                    )
                    .Nonblocking();
            var owner = new ObjectId(Guid.NewGuid());
            void Send(Batch batch)
            {
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
            }
            void Control(bool paused) =>
                Send(
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        Array.Empty<ParallelCommandGroup<Command>>(),
                        Start: BatchStart.Now
                    )
                    {
                        PresentationControl = new PresentationControl(17, owner, paused),
                    }
                );
            Send(
                new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    new[] { new ParallelCommandGroup<Command>(new[] { Play(37), Burst() }) },
                    WorkScope: 17
                )
            );
            for (int frame = 0; frame < 12; frame++)
                Frame();
            Assert.That(
                systems.Select(system => system.randomSeed),
                Is.EqualTo(new uint[] { 37, 38 })
            );
            var expected = ParticleState(systems[0]);
            Assert.That(expected, Is.Not.Empty);
            GameObject pooled = Spawned(burst).Single();
            Assert.That(pooled.activeSelf, Is.True);
            var burstExpected = ParticleState(pooled.GetComponent<ParticleSystem>());
            Assert.That(burstExpected, Is.Not.Empty);
            Frame(advance: false);
            Assert.That(ParticleState(systems[0]), Is.EqualTo(expected));
            Control(true);
            for (int frame = 0; frame < 30; frame++)
                Frame();
            Assert.That(ParticleState(systems[0]), Is.EqualTo(expected));
            Assert.That(
                ParticleState(pooled.GetComponent<ParticleSystem>()),
                Is.EqualTo(burstExpected)
            );
            Assert.That(harness.Runner.ObserveDittoWork().HasHeldOperations, Is.True);
            Control(false);
            Submit(harness, session, Play(37));
            for (int frame = 0; frame < 12; frame++)
                Frame();
            Assert.That(ParticleState(systems[0]), Is.EqualTo(expected));
            Submit(harness, session, Play(91));
            for (int frame = 0; frame < 12; frame++)
                Frame();
            Assert.That(ParticleState(systems[0]), Is.Not.EqualTo(expected));
            Assert.That(pooled.activeSelf, Is.False);
            Submit(harness, session, Burst());
            for (int frame = 0; frame < 12; frame++)
                Frame();
            Assert.That(Spawned(burst).Single(), Is.SameAs(pooled));
            Assert.That(
                ParticleState(pooled.GetComponent<ParticleSystem>()),
                Is.EqualTo(burstExpected)
            );
            Submit(
                harness,
                session,
                Command(new CommandBody.Particle.Stop(objectId, Clear: false))
            );
            for (int frame = 0; frame < 180; frame++)
                Frame();
            Assert.That(systems.Sum(system => system.particleCount), Is.Zero);
            Assert.That(harness.Runner.ObserveDittoWork().HasInfiniteOperations, Is.False);
            Assert.That(harness.Runner.ObserveDittoWork().HasPendingWork, Is.False);
        }

        private static (uint, UVector3, float)[] ParticleState(ParticleSystem system)
        {
            var particles = new ParticleSystem.Particle[system.particleCount];
            system.GetParticles(particles);
            return particles
                .Select(particle =>
                    (particle.randomSeed, particle.position, particle.remainingLifetime)
                )
                .ToArray();
        }

        [Test]
        public void BlockingSpawnUsesBothLocationsAndReleasesNonpooledInstances()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
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
            Assert.That(
                harness.Transport.Calls.Last(),
                Is.Not.EqualTo("stop"),
                string.Join("\n", harness.Logger.Records.Select(record => record.Message))
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
                Command(new CommandBody.Assets.ReplaceSet(FixtureAssets(harness)))
            );
            Assert.That(
                harness.Runner.TryGetPreparedAsset(
                    new PreparedAsset.ParticleEffect(address),
                    out _
                ),
                Is.False
            );

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
            Assert.That(Failures(harness), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        [Test]
        public void PoolReusesInComponentOrderEnforcesCapAndRetainsLeasesUntilCleared()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var address = new ParticleEffectAddress("game/pooled");
            GameObject prefab = ParticlePrefab(address.Value);
            prefab.AddComponent<BattlementEffectPool>().MaxInactiveCount = 1;
            prefab.AddComponent<PoolResetRecorder>().Label = "first";
            prefab.AddComponent<PoolResetRecorder>().Label = "second";
            harness.AssetStorage.EnqueueValue(prefab);
            PreparedAsset effect = new PreparedAsset.ParticleEffect(address);
            SessionId session = Connect(
                harness,
                new[] { effect },
                Array.Empty<BattlementGameObject>()
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

            var snapshot = FakeBattlementTransport.CompleteSnapshot(session);
            var response = new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 2 });

            Assert.That(Spawned(prefab), Is.Empty);
            Assert.That(harness.Runner.TryGetPreparedAsset(effect, out _), Is.False);
        }

        [Test]
        public void ResetExceptionDestroysTheInstanceAndReportsTheOperationFailure()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var address = new ParticleEffectAddress("game/broken-reset");
            GameObject prefab = ParticlePrefab(address.Value);
            prefab.AddComponent<BattlementEffectPool>().MaxInactiveCount = 2;
            prefab.AddComponent<ThrowingPoolReset>();
            harness.AssetStorage.EnqueueValue(prefab);
            SessionId session = Connect(
                harness,
                new PreparedAsset[] { new PreparedAsset.ParticleEffect(address) },
                Array.Empty<BattlementGameObject>()
            );
            Command spawn = Spawn(address, 10).Nonblocking();

            Submit(harness, session, spawn);
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
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
            BattlementTestHarness harness,
            IReadOnlyList<PreparedAsset> assets,
            IReadOnlyList<BattlementGameObject> objects
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    preparedAssets: assets,
                    objects: objects
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static BattlementGameObject PrefabObject(ObjectId id, PrefabAddress address) =>
            new(
                id,
                new GameObjectKind.Prefab(address, Array.Empty<MaterialAssignment>(), null),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static BattlementGameObject Empty(ObjectId id, Vector3? position = null) =>
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

        private static PreparedAsset[] FixtureAssets(BattlementTestHarness harness) =>
            harness
                .AssetStorage.PrepareCalls.Where(FakeBattlementTransport.IsFixtureAsset)
                .ToArray();

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params Command[] commands
        ) => Submit(harness, session, commands, reportsFailure: false);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            Command command,
            bool reportsFailure
        ) => Submit(harness, session, new[] { command }, reportsFailure);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            Command[] commands,
            bool reportsFailure
        ) => SubmitGroups(harness, session, commands, reportsFailure: reportsFailure);

        private static void SubmitGroups(
            BattlementTestHarness harness,
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
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            if (reportsFailure)
            {
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void Advance(BattlementTestHarness harness, double milliseconds)
        {
            harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
            harness.Runner.RunFrame();
        }

        private static GameObject Find(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .Single(value => value.Id == id.Value)
                .gameObject;

        private static bool HasIdentity(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
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

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness.Transport.BatchFailures.ToArray();

        private static OperationFailed<CoreErrorCode>[] OperationFailures(
            BattlementTestHarness harness
        ) => harness.Transport.OperationFailures.ToArray();
    }

    public sealed class PoolResetRecorder : MonoBehaviour, IBattlementPoolReset
    {
        public static List<(string Action, GameObject Instance)> Events { get; } = new();

        public string Label = string.Empty;

        public void OnBattlementAcquire() => Events.Add(($"acquire:{Label}", gameObject));

        public void OnBattlementRelease() => Events.Add(($"release:{Label}", gameObject));
    }

    public sealed class ThrowingPoolReset : MonoBehaviour, IBattlementPoolReset
    {
        public void OnBattlementAcquire() { }

        public void OnBattlementRelease() =>
            throw new InvalidOperationException("release reset failed");
    }
}
