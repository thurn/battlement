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
                    true,
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
            IReadOnlyList<MasonryGameObject>? objects = null
        )
        {
            SessionId session = responseSession ?? new SessionId(Guid.NewGuid());
            var snapshot = new Snapshot(
                snapshotSession ?? session,
                preparedAssets ?? Array.Empty<PreparedAsset>(),
                Array.Empty<MasonryScene>(),
                objects ?? Array.Empty<MasonryGameObject>(),
                new ObjectId(Guid.NewGuid()),
                null,
                inputDisabled,
                Array.Empty<KeyCode>()
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

        public static MasonryTransportResult ResponseResult(Response response) =>
            new(MasonryTransportStatus.Success, MasonryMessagePack.SerializeResponse(response));
    }

    internal sealed class FakeMasonryAssetStorage : IMasonryAssetStorage
    {
        private readonly HashSet<FakeAssetHandle> handles = new();
        private readonly Queue<Action<FakeAssetHandle>> preparations = new();

        public int LiveHandleCount => handles.Count;

        public List<PreparedAsset> PrepareCalls { get; } = new();

        public IReadOnlyCollection<FakeAssetHandle> Handles => handles;

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

        public void Dispose()
        {
            foreach (FakeAssetHandle handle in handles.ToArray())
            {
                handle.Dispose();
            }

            IsDisposed = true;
        }

        private void Remove(FakeAssetHandle handle) => handles.Remove(handle);
    }

    internal sealed class FakeAssetHandle : IMasonryAssetHandle
    {
        private readonly Action<FakeAssetHandle> onDispose;
        private bool isDisposed;

        public FakeAssetHandle(PreparedAsset asset, Action<FakeAssetHandle> onDispose)
        {
            Asset = asset;
            Value = asset;
            this.onDispose = onDispose;
        }

        public PreparedAsset Asset { get; }

        public bool IsDone { get; private set; } = true;

        public object? Value { get; private set; }

        public Exception? Error { get; private set; }

        public bool IsDisposed => isDisposed;

        public void Complete(object? value = null)
        {
            Value = value ?? Asset;
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
            onDispose(this);
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
