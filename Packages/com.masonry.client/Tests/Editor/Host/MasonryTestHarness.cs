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

        private MasonryTestHarness(Scene scene, GameObject hostObject, MasonryRunner runner)
        {
            Scene = scene;
            this.hostObject = hostObject;
            Runner = runner;
            Transport = new FakeMasonryTransport();
            AssetStorage = new FakeMasonryAssetStorage();
            Clock = new FakeMasonryClock();
            Logger = new FakeMasonryLogger();
            Runner.Configure(
                new MasonryRunnerOptions(Transport, AssetStorage, Clock, Logger, true)
            );
        }

        public Scene Scene { get; }

        public MasonryRunner Runner { get; }

        public FakeMasonryTransport Transport { get; }

        public FakeMasonryAssetStorage AssetStorage { get; }

        public FakeMasonryClock Clock { get; }

        public FakeMasonryLogger Logger { get; }

        public static MasonryTestHarness Create()
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            scene.name = $"Masonry test {Guid.NewGuid():N}";
            var hostObject = new GameObject("Masonry host");
            SceneManager.MoveGameObjectToScene(hostObject, scene);
            MasonryRunner runner = hostObject.AddComponent<MasonryRunner>();
            return new MasonryTestHarness(scene, hostObject, runner);
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
        public List<string> Calls { get; } = new();

        public bool IsDisposed { get; private set; }

        public void Connect() => Calls.Add("connect");

        public void Stop() => Calls.Add("stop");

        public void Dispose() => IsDisposed = true;
    }

    internal sealed class FakeMasonryAssetStorage : IMasonryAssetStorage
    {
        private readonly HashSet<FakeAssetHandle> handles = new();

        public int LiveHandleCount => handles.Count;

        public bool IsDisposed { get; private set; }

        public IMasonryAssetHandle Prepare(PreparedAsset asset)
        {
            var handle = new FakeAssetHandle(asset, Remove);
            handles.Add(handle);
            return handle;
        }

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
            this.onDispose = onDispose;
        }

        public PreparedAsset Asset { get; }

        public bool IsDone => true;

        public object? Value => Asset;

        public Exception? Error => null;

        public bool IsDisposed => isDisposed;

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
