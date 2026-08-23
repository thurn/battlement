#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    internal sealed class BattlementTestHarness : IDisposable
    {
        private readonly GameObject hostObject;
        private bool isDisposed;

        private BattlementTestHarness(
            Scene scene,
            GameObject hostObject,
            BattlementRunner runner,
            BattlementTransportKind transportKind,
            bool useInstantAnimations,
            IEnumerable<string>? customCommandTypes,
            IBattlementProtocolCodec? protocolCodec,
            IBattlementIncidentSink? incidentSink,
            IBattlementFailurePresenter? failurePresenter,
            bool suppressDevelopmentErrorDialogs
        )
        {
            Scene = scene;
            this.hostObject = hostObject;
            Runner = runner;
            Transport = new FakeBattlementTransport();
            Transport.Kind = transportKind;
            AssetStorage = new FakeBattlementAssetStorage();
            Clock = new FakeBattlementClock();
            Logger = new FakeBattlementLogger();
            IncidentSink = incidentSink ?? new FakeBattlementIncidentSink();
            Runner.Configure(
                new BattlementRunnerOptions(
                    Transport,
                    AssetStorage,
                    protocolCodec ?? BattlementJson.Instance,
                    Clock,
                    Logger,
                    useInstantAnimations,
                    customCommandTypes,
                    IncidentSink,
                    failurePresenter,
                    suppressDevelopmentErrorDialogs
                )
            );
        }

        public Scene Scene { get; }

        public BattlementRunner Runner { get; }

        public FakeBattlementTransport Transport { get; }

        public FakeBattlementAssetStorage AssetStorage { get; }

        public FakeBattlementClock Clock { get; }

        public FakeBattlementLogger Logger { get; }

        public IBattlementIncidentSink IncidentSink { get; }

        public static BattlementTestHarness Create(
            BattlementTransportKind transportKind = BattlementTransportKind.Native,
            bool useInstantAnimations = true,
            IEnumerable<string>? customCommandTypes = null,
            IBattlementProtocolCodec? protocolCodec = null,
            IBattlementIncidentSink? incidentSink = null,
            IBattlementFailurePresenter? failurePresenter = null,
            bool suppressDevelopmentErrorDialogs = true
        )
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            scene.name = $"Battlement test {Guid.NewGuid():N}";
            var hostObject = new GameObject("Battlement host");
            SceneManager.MoveGameObjectToScene(hostObject, scene);
            BattlementRunner runner = hostObject.AddComponent<BattlementRunner>();
            return new BattlementTestHarness(
                scene,
                hostObject,
                runner,
                transportKind,
                useInstantAnimations,
                customCommandTypes,
                protocolCodec,
                incidentSink,
                failurePresenter,
                suppressDevelopmentErrorDialogs
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

    internal sealed class FakeBattlementTransport : IBattlementTransport
    {
        private static readonly SceneId DefaultSceneId = new(
            Guid.Parse("00000000-0000-0000-0000-000000000101")
        );
        private static readonly ObjectId DefaultInputCameraId = new(
            Guid.Parse("00000000-0000-0000-0000-000000000102")
        );
        private static readonly SceneAddress DefaultSceneAddress = new(
            "battlement/tests/default-scene"
        );

        private readonly Queue<BattlementTransportResult> connectResults = new();
        private readonly Queue<BattlementTransportResult> submitResults = new();
        private readonly Queue<BattlementTransportResult> pollResults = new();

        public BattlementTransportKind Kind { get; set; } = BattlementTransportKind.Native;

        public List<string> Calls { get; } = new();

        public List<byte[]> ConnectMessages { get; } = new();

        public List<byte[]> SubmitMessages { get; } = new();

        public BattlementTransportResult? DefaultSubmitResult { get; set; }

        public bool IsDisposed { get; private set; }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json)
        {
            Calls.Add("connect");
            ConnectMessages.Add(json.ToArray());
            return connectResults.Count > 0 ? connectResults.Dequeue() : SnapshotResponse();
        }

        public void EnqueueConnect(BattlementTransportResult result) =>
            connectResults.Enqueue(result);

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> json)
        {
            Calls.Add("submit");
            SubmitMessages.Add(json.ToArray());
            return submitResults.Count > 0
                ? submitResults.Dequeue()
                : DefaultSubmitResult
                    ?? new BattlementTransportResult(BattlementTransportStatus.Success, json);
        }

        public void EnqueueSubmit(BattlementTransportResult result) =>
            submitResults.Enqueue(result);

        public BattlementTransportResult Poll()
        {
            Calls.Add("poll");
            return pollResults.Count > 0
                ? pollResults.Dequeue()
                : new BattlementTransportResult(BattlementTransportStatus.NoMessage);
        }

        public void EnqueuePoll(BattlementTransportResult result) => pollResults.Enqueue(result);

        public void Stop() => Calls.Add("stop");

        public void Dispose() => IsDisposed = true;

        public static BattlementTransportResult SnapshotResponse(
            SessionId? responseSession = null,
            SessionId? snapshotSession = null,
            bool inputDisabled = false,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<BattlementScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<BattlementGameObject>? objects = null,
            ObjectId? inputCameraId = null,
            IReadOnlyList<KeyCode>? globalKeys = null,
            bool useMainCamera = false,
            ControllerInputSettings? controllerInput = null
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
                inputDisabled,
                globalKeys,
                useMainCamera,
                controllerInput
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
            return new BattlementTransportResult(
                BattlementTransportStatus.Success,
                BattlementJson.SerializeResponse(response)
            );
        }

        public static Snapshot CompleteSnapshot(
            SessionId session,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<BattlementScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<BattlementGameObject>? objects = null,
            ObjectId? inputCameraId = null,
            bool inputDisabled = false,
            IReadOnlyList<KeyCode>? globalKeys = null,
            bool useMainCamera = false,
            ControllerInputSettings? controllerInput = null
        )
        {
            var completedAssets = new List<PreparedAsset>(
                preparedAssets ?? Array.Empty<PreparedAsset>()
            );
            PreparedAsset.Scene? preparedScene = completedAssets
                .OfType<PreparedAsset.Scene>()
                .FirstOrDefault();
            SceneAddress fixtureSceneAddress = preparedScene?.Address ?? DefaultSceneAddress;
            IReadOnlyList<BattlementScene> completedScenes =
                scenes ?? new[] { new BattlementScene(DefaultSceneId, fixtureSceneAddress) };
            if (scenes is null)
            {
                if (!completedAssets.OfType<PreparedAsset.Scene>().Any())
                {
                    completedAssets.Add(new PreparedAsset.Scene(DefaultSceneAddress));
                }
            }

            var completedObjects = new List<BattlementGameObject>(
                objects ?? Array.Empty<BattlementGameObject>()
            );
            if (useMainCamera && inputCameraId is not null)
            {
                throw new ArgumentException(
                    "A fixture snapshot cannot select both main and Battlement cameras."
                );
            }

            ObjectId? completedCamera = useMainCamera
                ? null
                : inputCameraId ?? DefaultInputCameraId;
            if (!useMainCamera && inputCameraId is null)
            {
                completedObjects.Add(
                    new BattlementGameObject(
                        completedCamera!.Value,
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
                globalKeys ?? Array.Empty<KeyCode>(),
                controllerInput
            );
        }

        public static bool IsFixtureAsset(PreparedAsset asset) =>
            asset is PreparedAsset.Scene { Address: { Value: "battlement/tests/default-scene" } };

        public static bool IsFixtureIdentity(BattlementIdentity identity) =>
            identity.Id == DefaultInputCameraId.Value;

        public static BattlementTransportResult ResponseResult(Response response)
        {
            byte[] bytes = BattlementJson.SerializeResponse(response);
            return new(BattlementTransportStatus.Success, bytes);
        }
    }

    internal sealed class FakeBattlementAssetStorage : IBattlementAssetStorage
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

        public IBattlementAssetHandle Prepare(PreparedAsset asset)
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

        public IBattlementSceneHandle LoadScene(IBattlementAssetLease sceneAsset)
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
                    "Packages/com.battlement.client/Tests/Fixtures/Scenes/"
                    + $"ContentScene{suffix}.unity";
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

    internal sealed class FakeSceneHandle : IBattlementSceneHandle
    {
        private readonly IBattlementAssetLease lease;
        private readonly Action<FakeSceneHandle> onDispose;
        private bool unloadPending;
        private bool isDisposed;

        public FakeSceneHandle(
            IBattlementAssetLease lease,
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

    internal sealed class FakeAssetHandle : IBattlementAssetHandle
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

    internal sealed class FakeBattlementClock : IBattlementClock
    {
        public TimeSpan Elapsed { get; private set; }

        public void Advance(TimeSpan duration) => Elapsed += duration;
    }

    internal sealed class FakeBattlementLogger : IBattlementLogger
    {
        public List<BattlementLogRecord> Records { get; } = new();

        public void Log(BattlementLogRecord record) => Records.Add(record);
    }
}
