#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    internal sealed class MasonryTestHarness : IDisposable
    {
        private readonly GameObject hostObject;
        private bool isDisposed;

        private MasonryTestHarness(
            Scene scene,
            GameObject hostObject,
            MasonryRunner runner,
            MasonryTransportKind transportKind,
            bool useInstantAnimations,
            IEnumerable<string>? customCommandTypes,
            IMasonryProtocolCodec? protocolCodec
        )
        {
            Scene = scene;
            this.hostObject = hostObject;
            Runner = runner;
            Transport = new FakeMasonryTransport();
            Transport.Kind = transportKind;
            AssetStorage = new FakeMasonryAssetStorage();
            Clock = new FakeMasonryClock();
            Logger = new FakeMasonryLogger();
            Runner.Configure(
                new MasonryRunnerOptions(
                    Transport,
                    AssetStorage,
                    protocolCodec ?? MasonryMessagePack.Instance,
                    Clock,
                    Logger,
                    useInstantAnimations,
                    customCommandTypes
                )
            );
        }

        public Scene Scene { get; }

        public MasonryRunner Runner { get; }

        public FakeMasonryTransport Transport { get; }

        public FakeMasonryAssetStorage AssetStorage { get; }

        public FakeMasonryClock Clock { get; }

        public FakeMasonryLogger Logger { get; }

        public static MasonryTestHarness Create(
            MasonryTransportKind transportKind = MasonryTransportKind.Native,
            bool useInstantAnimations = true,
            IEnumerable<string>? customCommandTypes = null,
            IMasonryProtocolCodec? protocolCodec = null
        )
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            scene.name = $"Masonry test {Guid.NewGuid():N}";
            var hostObject = new GameObject("Masonry host");
            SceneManager.MoveGameObjectToScene(hostObject, scene);
            MasonryRunner runner = hostObject.AddComponent<MasonryRunner>();
            return new MasonryTestHarness(
                scene,
                hostObject,
                runner,
                transportKind,
                useInstantAnimations,
                customCommandTypes,
                protocolCodec
            );
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            Runner.Stop();
            Runner.Dispose();
            Object.DestroyImmediate(hostObject);
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            isDisposed = true;
        }
    }

    internal sealed class FakeMasonryTransport : IMasonryTransport
    {
        private static readonly SceneId DefaultSceneId = new(
            Guid.Parse("00000000-0000-0000-0000-000000000101")
        );
        private static readonly ObjectId DefaultInputCameraId = new(
            Guid.Parse("00000000-0000-0000-0000-000000000102")
        );
        private static readonly SceneAddress DefaultSceneAddress = new(
            "masonry/tests/default-scene"
        );

        private readonly Queue<MasonryTransportResult> connectResults = new();
        private readonly Queue<MasonryTransportResult> submitResults = new();
        private readonly Queue<MasonryTransportResult> pollResults = new();

        public MasonryTransportKind Kind { get; set; } = MasonryTransportKind.Native;

        public List<string> Calls { get; } = new();

        public List<byte[]> ConnectMessages { get; } = new();

        public List<byte[]> SubmitMessages { get; } = new();

        public bool IsDisposed { get; private set; }

        public MasonryTransportResult Connect(ReadOnlyMemory<byte> messagePack)
        {
            Calls.Add("connect");
            ConnectMessages.Add(messagePack.ToArray());
            return connectResults.Count > 0 ? connectResults.Dequeue() : SnapshotResponse();
        }

        public void EnqueueConnect(MasonryTransportResult result) => connectResults.Enqueue(result);

        public MasonryTransportResult Submit(ReadOnlyMemory<byte> messagePack)
        {
            Calls.Add("submit");
            SubmitMessages.Add(messagePack.ToArray());
            return submitResults.Count > 0
                ? submitResults.Dequeue()
                : new MasonryTransportResult(MasonryTransportStatus.Success, messagePack);
        }

        public void EnqueueSubmit(MasonryTransportResult result) => submitResults.Enqueue(result);

        public MasonryTransportResult Poll()
        {
            Calls.Add("poll");
            return pollResults.Count > 0
                ? pollResults.Dequeue()
                : new MasonryTransportResult(MasonryTransportStatus.NoMessage);
        }

        public void EnqueuePoll(MasonryTransportResult result) => pollResults.Enqueue(result);

        public void Stop() => Calls.Add("stop");

        public void Dispose() => IsDisposed = true;

        public static MasonryTransportResult SnapshotResponse(
            SessionId? responseSession = null,
            SessionId? snapshotSession = null,
            bool inputDisabled = false,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<MasonryScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<MasonryGameObject>? objects = null,
            ObjectId? inputCameraId = null
        )
        {
            SessionId session = responseSession ?? new SessionId(Guid.NewGuid());
            Snapshot snapshot = CompleteSnapshot(
                snapshotSession ?? session,
                preparedAssets,
                scenes,
                primarySceneId,
                objects,
                inputCameraId,
                inputDisabled
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
            return new MasonryTransportResult(
                MasonryTransportStatus.Success,
                MasonryMessagePack.SerializeResponse(response)
            );
        }

        public static Snapshot CompleteSnapshot(
            SessionId session,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<MasonryScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<MasonryGameObject>? objects = null,
            ObjectId? inputCameraId = null,
            bool inputDisabled = false,
            IReadOnlyList<KeyCode>? globalKeys = null
        )
        {
            var completedAssets = new List<PreparedAsset>(
                preparedAssets ?? Array.Empty<PreparedAsset>()
            );
            PreparedAsset.Scene? preparedScene = completedAssets
                .OfType<PreparedAsset.Scene>()
                .FirstOrDefault();
            SceneAddress fixtureSceneAddress = preparedScene?.Address ?? DefaultSceneAddress;
            IReadOnlyList<MasonryScene> completedScenes =
                scenes ?? new[] { new MasonryScene(DefaultSceneId, fixtureSceneAddress) };
            if (scenes is null)
            {
                if (!completedAssets.OfType<PreparedAsset.Scene>().Any())
                {
                    completedAssets.Add(new PreparedAsset.Scene(DefaultSceneAddress));
                }
            }

            var completedObjects = new List<MasonryGameObject>(
                objects ?? Array.Empty<MasonryGameObject>()
            );
            ObjectId completedCamera = inputCameraId ?? DefaultInputCameraId;
            if (inputCameraId is null)
            {
                completedObjects.Add(
                    new MasonryGameObject(
                        completedCamera,
                        new GameObjectKind.Camera(new CameraState()),
                        new ParentScene.Persistent(),
                        null,
                        true,
                        LocalTransform.Identity,
                        Array.Empty<PointerEvent>()
                    )
                );
            }

            return new Snapshot(
                session,
                completedAssets,
                completedScenes,
                completedObjects,
                completedCamera,
                primarySceneId,
                inputDisabled,
                globalKeys ?? Array.Empty<KeyCode>()
            );
        }

        public static bool IsFixtureAsset(PreparedAsset asset) =>
            asset is PreparedAsset.Scene { Address: { Value: "masonry/tests/default-scene" } };

        public static bool IsFixtureIdentity(MasonryIdentity identity) =>
            identity.Id == DefaultInputCameraId.Value;

        public static MasonryTransportResult ResponseResult(Response response) =>
            new(MasonryTransportStatus.Success, MasonryMessagePack.SerializeResponse(response));
    }

    internal sealed class FakeMasonryAssetStorage : IMasonryAssetStorage
    {
        private readonly HashSet<FakeAssetHandle> handles = new();
        private readonly HashSet<FakeSceneHandle> sceneHandles = new();
        private readonly Queue<Action<FakeAssetHandle>> preparations = new();
        private readonly Queue<Action<FakeSceneHandle>> sceneLoads = new();
        private readonly Dictionary<string, string> scenePaths = new(StringComparer.Ordinal);
        private int nextScenePath;

        public int LiveHandleCount => handles.Count;

        public List<PreparedAsset> PrepareCalls { get; } = new();

        public IReadOnlyCollection<FakeAssetHandle> Handles => handles;

        public IReadOnlyCollection<FakeSceneHandle> SceneHandles => sceneHandles;

        public List<PreparedAsset.Scene> SceneLoadCalls { get; } = new();

        public bool IsDisposed { get; private set; }

        public IMasonryAssetHandle Prepare(PreparedAsset asset)
        {
            var handle = new FakeAssetHandle(asset, Remove);
            handles.Add(handle);
            PrepareCalls.Add(asset);
            if (preparations.Count > 0)
            {
                preparations.Dequeue()(handle);
            }

            return handle;
        }

        public void EnqueuePending() => preparations.Enqueue(handle => handle.SetPending());

        public void EnqueueFailure(Exception error) =>
            preparations.Enqueue(handle => handle.SetFailure(error));

        public void EnqueueValue(object value) =>
            preparations.Enqueue(handle => handle.Complete(value));

        public IMasonrySceneHandle LoadScene(IMasonryAssetLease sceneAsset)
        {
            PreparedAsset.Scene asset = (PreparedAsset.Scene)sceneAsset.Asset;
            if (!scenePaths.TryGetValue(asset.Address.Value, out string path))
            {
                string suffix = nextScenePath++ switch
                {
                    0 => "A",
                    1 => "B",
                    _ => throw new InvalidOperationException(
                        "The scene fixture supports two simultaneous addresses."
                    ),
                };
                path =
                    $"Packages/com.masonry.client/Tests/Fixtures/Scenes/ContentScene{suffix}.unity";
                scenePaths.Add(asset.Address.Value, path);
            }

            var handle = new FakeSceneHandle(sceneAsset, path, Remove);
            sceneHandles.Add(handle);
            SceneLoadCalls.Add(handle.Asset);
            if (sceneLoads.Count > 0)
            {
                sceneLoads.Dequeue()(handle);
            }

            return handle;
        }

        public void EnqueueSceneLoadPending() =>
            sceneLoads.Enqueue(handle => handle.SetLoadPending());

        public void EnqueueSceneUnloadPending() =>
            sceneLoads.Enqueue(handle => handle.SetUnloadPending());

        public void EnqueueSceneFailure(Exception error) =>
            sceneLoads.Enqueue(handle => handle.SetFailure(error));

        public void Dispose()
        {
            foreach (FakeAssetHandle handle in handles.ToArray())
            {
                handle.Dispose();
            }

            foreach (FakeSceneHandle handle in sceneHandles.ToArray())
            {
                handle.Dispose();
            }

            IsDisposed = true;
        }

        private void Remove(FakeAssetHandle handle) => handles.Remove(handle);

        private void Remove(FakeSceneHandle handle) => sceneHandles.Remove(handle);
    }

    internal sealed class FakeSceneHandle : IMasonrySceneHandle
    {
        private readonly IMasonryAssetLease lease;
        private readonly Action<FakeSceneHandle> onDispose;
        private bool unloadPending;
        private bool isDisposed;

        public FakeSceneHandle(
            IMasonryAssetLease lease,
            string scenePath,
            Action<FakeSceneHandle> onDispose
        )
        {
            this.lease = lease;
            this.onDispose = onDispose;
            Asset = (PreparedAsset.Scene)lease.Asset;
            Scene = EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Additive);
        }

        public PreparedAsset.Scene Asset { get; }

        public bool IsLoaded { get; private set; } = true;

        public Scene Scene { get; }

        public Exception? Error { get; private set; }

        public bool IsUnloaded { get; private set; }

        public int UnloadCallCount { get; private set; }

        public void BeginUnload()
        {
            if (IsUnloaded || UnloadCallCount > 0)
            {
                return;
            }

            UnloadCallCount++;
            if (!unloadPending)
            {
                CompleteUnload();
            }
        }

        public void CompleteLoad()
        {
            IsLoaded = true;
            Error = null;
        }

        public void CompleteUnload()
        {
            if (IsUnloaded)
            {
                return;
            }

            if (Scene.IsValid() && Scene.isLoaded)
            {
                EditorSceneManager.CloseScene(Scene, true);
            }

            IsLoaded = false;
            IsUnloaded = true;
            lease.Dispose();
        }

        public void SetLoadPending() => IsLoaded = false;

        public void SetUnloadPending() => unloadPending = true;

        public void SetFailure(Exception error)
        {
            IsLoaded = false;
            Error = error;
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            BeginUnload();
            isDisposed = true;
            onDispose(this);
        }
    }

    internal sealed class FakeAssetHandle : IMasonryAssetHandle
    {
        private readonly Action<FakeAssetHandle> onDispose;
        private bool isDisposed;

        public FakeAssetHandle(PreparedAsset asset, Action<FakeAssetHandle> onDispose)
        {
            Asset = asset;
            Value = DefaultValue();
            if (Value is GameObject prefab)
            {
                prefab.SetActive(false);
            }

            this.onDispose = onDispose;
        }

        public PreparedAsset Asset { get; }

        public bool IsDone { get; private set; } = true;

        public object? Value { get; private set; }

        public Exception? Error { get; private set; }

        public bool IsDisposed => isDisposed;

        public void Complete(object? value = null)
        {
            if (
                value is not null
                && Value is GameObject previous
                && !ReferenceEquals(previous, value)
            )
            {
                Object.DestroyImmediate(previous);
            }

            Value = value ?? DefaultValue();
            if (Value is GameObject prefab)
            {
                prefab.SetActive(false);
            }

            Error = null;
            IsDone = true;
        }

        public void SetFailure(Exception error)
        {
            Value = null;
            Error = error;
            IsDone = true;
        }

        public void SetPending()
        {
            Value = null;
            Error = null;
            IsDone = false;
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            isDisposed = true;
            if (Value is GameObject prefab)
            {
                Object.DestroyImmediate(prefab);
            }

            onDispose(this);
        }

        private object DefaultValue() =>
            Asset switch
            {
                PreparedAsset.Prefab prefab => new GameObject($"Prepared {prefab.Address.Value}"),
                PreparedAsset.ParticleEffect effect => ParticleEffect(effect.Address),
                _ => Asset,
            };

        private static GameObject ParticleEffect(ParticleEffectAddress address)
        {
            var prefab = new GameObject($"Prepared {address.Value}");
            prefab.AddComponent<ParticleSystem>();
            return prefab;
        }
    }

    internal sealed class FakeMasonryClock : IMasonryClock
    {
        public TimeSpan Elapsed { get; private set; }

        public void Advance(TimeSpan duration) => Elapsed += duration;
    }

    internal sealed class FakeMasonryLogger : IMasonryLogger
    {
        public List<MasonryLogRecord> Records { get; } = new();

        public void Log(MasonryLogRecord record) => Records.Add(record);
    }
}
