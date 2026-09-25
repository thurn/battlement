#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.Errors;
using Google.FlatBuffers;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;
using Wire = Battlement.FlatBuffers.Generated;

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
            bool useInstantAnimations,
            IEnumerable<string>? customCommandTypes,
            IBattlementErrorSink? errorSink,
            IBattlementFailurePresenter? failurePresenter,
            bool suppressDevelopmentErrorDialogs,
            Action<string>? openExternalUrl,
            IBattlementFlatBufferResponseViewSchema? flatBufferResponseSchema,
            IBattlementFlatBufferClientSchema? flatBufferClientSchema,
            Func<HostSettings>? readHostSettings
        )
        {
            Scene = scene;
            this.hostObject = hostObject;
            Runner = runner;
            Transport = new FakeBattlementTransport();
            AssetStorage = new FakeBattlementAssetStorage();
            Clock = new FakeBattlementClock();
            Logger = new FakeBattlementLogger();
            ErrorSink = errorSink ?? new FakeBattlementErrorSink();
            Runner.Configure(
                new BattlementRunnerOptions(
                    Transport,
                    AssetStorage,
                    clock: Clock,
                    logger: Logger,
                    useInstantAnimations: useInstantAnimations,
                    customCommandTypes: customCommandTypes,
                    errorSink: ErrorSink,
                    failurePresenter: failurePresenter,
                    suppressDevelopmentErrorDialogs: suppressDevelopmentErrorDialogs,
                    caughtFailureReporter: new FakeCaughtFailureReporter(),
                    openExternalUrl: openExternalUrl,
                    flatBufferResponseSchema: flatBufferResponseSchema,
                    flatBufferClientSchema: flatBufferClientSchema,
                    readHostSettings: readHostSettings ?? (() => new HostSettings())
                )
            );
        }

        public Scene Scene { get; }

        public BattlementRunner Runner { get; }

        public FakeBattlementTransport Transport { get; }

        public FakeBattlementAssetStorage AssetStorage { get; }

        public FakeBattlementClock Clock { get; }

        public FakeBattlementLogger Logger { get; }

        public IBattlementErrorSink ErrorSink { get; }

        public static BattlementTestHarness Create(
            bool useInstantAnimations = true,
            IEnumerable<string>? customCommandTypes = null,
            IBattlementErrorSink? errorSink = null,
            IBattlementFailurePresenter? failurePresenter = null,
            bool suppressDevelopmentErrorDialogs = true,
            Action<string>? openExternalUrl = null,
            IBattlementFlatBufferResponseViewSchema? flatBufferResponseSchema = null,
            IBattlementFlatBufferClientSchema? flatBufferClientSchema = null,
            Func<HostSettings>? readHostSettings = null
        )
        {
            string sceneName = $"Battlement test {Guid.NewGuid():N}";
            Scene scene = Application.isPlaying
                ? SceneManager.CreateScene(sceneName)
                : EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            if (!Application.isPlaying)
                scene.name = sceneName;
            var hostObject = new GameObject("Battlement host");
            SceneManager.MoveGameObjectToScene(hostObject, scene);
            BattlementRunner runner = hostObject.AddComponent<BattlementRunner>();
            return new BattlementTestHarness(
                scene,
                hostObject,
                runner,
                useInstantAnimations,
                customCommandTypes,
                errorSink,
                failurePresenter,
                suppressDevelopmentErrorDialogs,
                openExternalUrl,
                flatBufferResponseSchema,
                flatBufferClientSchema,
                readHostSettings
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
            if (Application.isPlaying)
                SceneManager.UnloadSceneAsync(Scene);
            else
                EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            isDisposed = true;
        }
    }

    internal sealed class CallbackResponseView : IBattlementResponseView
    {
        private readonly IBattlementResponseView inner;
        private System.Action? beforeFirstRead;

        public CallbackResponseView(IBattlementResponseView inner, System.Action beforeFirstRead) =>
            (this.inner, this.beforeFirstRead) = (
                inner ?? throw new ArgumentNullException(nameof(inner)),
                beforeFirstRead ?? throw new ArgumentNullException(nameof(beforeFirstRead))
            );

        public SessionId SessionId
        {
            get
            {
                InvokeCallback();
                return inner.SessionId;
            }
        }

        public int MessageCount => inner.MessageCount;

        public bool IsSnapshot(int index) => inner.IsSnapshot(index);

        public IBattlementSnapshotView ReadSnapshot(int index) => inner.ReadSnapshot(index);

        public IBattlementBatchView ReadBatch(int index) => inner.ReadBatch(index);

        public void Dispose() => inner.Dispose();

        private void InvokeCallback()
        {
            System.Action? callback = beforeFirstRead;
            beforeFirstRead = null;
            callback?.Invoke();
        }
    }

    internal sealed class FakeCaughtFailureReporter : IBattlementCaughtFailureReporter
    {
        public List<BattlementError> Errors { get; } = new();

        public void Report(BattlementError error) => Errors.Add(error);
    }

    internal sealed class FakeBattlementTransport
        : IBattlementTransport,
            IBattlementCoreMessageObserver,
            IBattlementClientMessageObserver
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
        private readonly Queue<BattlementUiEventTransportResult> uiEventResults = new();
        private readonly Queue<BattlementTransportResult> pollResults = new();
        private SessionId? session;

        public List<string> Calls { get; } = new();

        public List<byte[]> ConnectMessages { get; } = new();

        public List<byte[]> SubmitMessages { get; } = new();

        public List<byte[]> UiEventMessages { get; } = new();

        public List<Connect> ConnectValues { get; } = new();

        public List<Action> Actions { get; } = new();

        public List<UiEventAction> UiEventActions { get; } = new();

        public List<BatchFailed<CoreErrorCode>> BatchFailures { get; } = new();

        public List<OperationFailed<CoreErrorCode>> OperationFailures { get; } = new();

        public Func<BattlementTransportResult>? DefaultSubmitResult { get; set; }

        public bool IsDisposed { get; private set; }

        public int DisposeCount { get; private set; }

        public BattlementTransportResult Connect(ReadOnlyMemory<byte> json)
        {
            Calls.Add("connect");
            ConnectMessages.Add(json.ToArray());
            BattlementTransportResult result =
                connectResults.Count > 0 ? connectResults.Dequeue() : SnapshotResponse();
            session = ReadSession(result.Payload) ?? session;
            return result;
        }

        public void EnqueueConnect(BattlementTransportResult result)
        {
            session = ReadSession(result.Payload);
            connectResults.Enqueue(result);
        }

        public BattlementTransportResult Submit(ReadOnlyMemory<byte> json)
        {
            Calls.Add("submit");
            SubmitMessages.Add(json.ToArray());
            return submitResults.Count > 0
                ? submitResults.Dequeue()
                : DefaultSubmitResult?.Invoke() ?? EmptyResponse();
        }

        public void EnqueueSubmit(BattlementTransportResult result) =>
            submitResults.Enqueue(result);

        public BattlementUiEventTransportResult SubmitUiEvent(ReadOnlyMemory<byte> message)
        {
            Calls.Add("submit_ui_event");
            byte[] bytes = message.ToArray();
            UiEventMessages.Add(bytes);
            if (uiEventResults.Count > 0)
            {
                return uiEventResults.Dequeue();
            }
            UiEventAction action = UiEventActions[^1];
            var response = new Response(action.SessionId, Array.Empty<ResponseMessage<Command>>());
            return new BattlementUiEventTransportResult(
                BattlementTransportStatus.Success,
                action.Event.DefaultPrevented
                    ? UiEventDisposition.PreventDefault
                    : UiEventDisposition.Continue,
                BattlementFlatBufferResponseFixtures.Write(response)
            );
        }

        public void EnqueueUiEvent(BattlementUiEventTransportResult result) =>
            uiEventResults.Enqueue(result);

        public void RecordConnect(Connect value) => ConnectValues.Add(value);

        public void RecordAction(Action value) => Actions.Add(value);

        public void RecordUiEvent(UiEventAction value) => UiEventActions.Add(value);

        public void RecordBatchFailure(BatchFailed<CoreErrorCode> value) =>
            BatchFailures.Add(value);

        public void RecordOperationFailure(OperationFailed<CoreErrorCode> value) =>
            OperationFailures.Add(value);

        public BattlementTransportResult Poll()
        {
            Calls.Add("poll");
            return pollResults.Count > 0
                ? pollResults.Dequeue()
                : new BattlementTransportResult(BattlementTransportStatus.NoMessage);
        }

        public void EnqueuePoll(BattlementTransportResult result) => pollResults.Enqueue(result);

        public void Stop() => Calls.Add("stop");

        public void Dispose()
        {
            DisposeCount++;
            IsDisposed = true;
        }

        public static BattlementTransportResult SnapshotResponse(
            SessionId? responseSession = null,
            SessionId? snapshotSession = null,
            bool inputDisabled = false,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<BattlementScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<BattlementGameObject>? objects = null,
            ObjectId? inputCameraId = null,
            IReadOnlyList<PhysicalKey>? globalKeys = null,
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
            return ResponseResult(response);
        }

        public static Snapshot CompleteSnapshot(
            SessionId session,
            IReadOnlyList<PreparedAsset>? preparedAssets = null,
            IReadOnlyList<BattlementScene>? scenes = null,
            SceneId? primarySceneId = null,
            IReadOnlyList<BattlementGameObject>? objects = null,
            ObjectId? inputCameraId = null,
            bool inputDisabled = false,
            IReadOnlyList<PhysicalKey>? globalKeys = null,
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
                globalKeys ?? Array.Empty<PhysicalKey>(),
                controllerInput
            );
        }

        public static bool IsFixtureAsset(PreparedAsset asset) =>
            asset is PreparedAsset.Scene { Address: { Value: "battlement/tests/default-scene" } };

        public static bool IsFixtureIdentity(BattlementIdentity identity) =>
            identity.Id == DefaultInputCameraId.Value;

        public static BattlementTransportResult ResponseResult(Response response) =>
            new(
                BattlementTransportStatus.Success,
                BattlementFlatBufferResponseFixtures.Write(response)
            );

        private static SessionId? ReadSession(ReadOnlyMemory<byte> payload)
        {
            if (payload.IsEmpty)
                return null;
            try
            {
                var bytes = new ByteBuffer(payload.ToArray());
                bytes.Position = FlatBufferConstants.SizePrefixLength;
                Wire.Response response = Wire.Response.GetRootAsResponse(bytes);
                return new SessionId(
                    BattlementFlatBufferCore.ReadUuid(response.SessionId, "response session")
                );
            }
            catch
            {
                return null;
            }
        }

        private BattlementTransportResult EmptyResponse()
        {
            SessionId current =
                Actions.Count > 0
                    ? Actions[^1].SessionId
                    : session
                        ?? throw new InvalidOperationException(
                            "A fake transport cannot answer before a session response is queued."
                        );
            return ResponseResult(new Response(current, Array.Empty<ResponseMessage<Command>>()));
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

        public int DisposeCount { get; private set; }

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
            DisposeCount++;
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
            Scene = Application.isPlaying
                ? EditorSceneManager.LoadSceneInPlayMode(
                    scenePath,
                    new LoadSceneParameters(LoadSceneMode.Additive)
                )
                : EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Additive);
        }

        public PreparedAsset.Scene Asset { get; }

        private bool loadCompleted = true;

        public bool IsLoaded
        {
            get => loadCompleted && (!Application.isPlaying || Scene.isLoaded);
            private set => loadCompleted = value;
        }

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
                if (Application.isPlaying)
                    SceneManager.UnloadSceneAsync(Scene);
                else
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
