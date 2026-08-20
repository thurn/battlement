#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Masonry
{
    internal sealed class MasonryScenes : IDisposable
    {
        private const int MaximumScenes = 32;

        private readonly IMasonryAssetStorage storage;
        private readonly MasonryPreparedAssets preparedAssets;
        private readonly MasonryWorld world;
        private readonly Dictionary<Guid, Entry> loaded = new();
        private readonly List<Entry> unloading = new();
        private Replacement? pending;
        private bool isDisposed;

        public MasonryScenes(
            IMasonryAssetStorage storage,
            MasonryPreparedAssets preparedAssets,
            MasonryWorld world
        )
        {
            this.storage = storage;
            this.preparedAssets = preparedAssets;
            this.world = world;
        }

        public void BeginSession()
        {
            ThrowIfDisposed();
            CancelPendingLoads();
            foreach (Entry entry in loaded.Values.ToArray())
            {
                BeginUnload(entry);
            }

            loaded.Clear();
        }

        public void BeginReplacement(IReadOnlyList<MasonryScene> scenes, SceneId? primarySceneId)
        {
            ThrowIfDisposed();
            Errors.CheckNotNull(scenes, nameof(scenes));
            if (pending is not null)
            {
                throw Failure("A scene replacement is already in progress.");
            }

            Replacement replacement = Validate(scenes, primarySceneId);
            foreach (Entry entry in loaded.Values.ToArray())
            {
                if (
                    replacement.Desired.TryGetValue(entry.Id, out MasonryScene desired)
                    && desired.Address == entry.Address
                )
                {
                    replacement.Retained.Add(entry.Id, entry);
                    continue;
                }

                BeginUnload(entry);
                loaded.Remove(entry.Id);
            }

            pending = replacement;
        }

        public bool TryCompleteReplacement(out MasonryAssetException? error)
        {
            ThrowIfDisposed();
            error = null;
            if (pending is null)
            {
                return true;
            }

            foreach (Entry entry in unloading.ToArray())
            {
                if (!entry.Handle.IsUnloaded)
                {
                    return false;
                }

                unloading.Remove(entry);
                entry.Handle.Dispose();
                if (entry.Handle.Error is Exception unloadError)
                {
                    error = Failure(
                        $"Scene '{entry.Address.Value}' failed to unload: {unloadError.Message}",
                        unloadError
                    );
                    CancelPendingLoads();
                    return true;
                }
            }

            try
            {
                StartPendingLoads();
                foreach (Entry entry in pending.Loading)
                {
                    if (entry.Handle.Error is Exception loadError)
                    {
                        throw Failure(
                            $"Scene '{entry.Address.Value}' failed to load: {loadError.Message}",
                            loadError
                        );
                    }

                    if (!entry.Handle.IsLoaded)
                    {
                        return false;
                    }
                }

                foreach (Entry entry in pending.Loading)
                {
                    world.RegisterScene(new SceneId(entry.Id), entry.Handle.Scene);
                    pending.Retained.Add(entry.Id, entry);
                }

                if (pending.PrimaryId is Guid primaryId)
                {
                    world.SetPrimaryScene(new SceneId(primaryId));
                }

                loaded.Clear();
                foreach ((Guid id, Entry entry) in pending.Retained)
                {
                    loaded.Add(id, entry);
                }

                pending = null;
                return true;
            }
            catch (Exception exception)
            {
                error =
                    exception as MasonryAssetException
                    ?? Failure($"Scene replacement failed: {exception.Message}", exception);
                CancelPendingLoads();
                return true;
            }
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            CancelPendingLoads();
            foreach (Entry entry in loaded.Values.ToArray())
            {
                BeginUnload(entry);
                entry.Handle.Dispose();
            }

            loaded.Clear();
            foreach (Entry entry in unloading)
            {
                entry.Handle.Dispose();
            }

            unloading.Clear();
            isDisposed = true;
        }

        private Replacement Validate(IReadOnlyList<MasonryScene> scenes, SceneId? primarySceneId)
        {
            if (scenes.Count > MaximumScenes)
            {
                throw new MasonryAssetException(
                    CoreErrorCode.LimitExceeded,
                    $"Masonry cannot load more than {MaximumScenes} content scenes."
                );
            }

            var desired = new Dictionary<Guid, MasonryScene>();
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            foreach (MasonryScene scene in scenes)
            {
                Guid id = scene.Id.Value;
                string address = scene.Address.Value;
                if (id == Guid.Empty || string.IsNullOrEmpty(address))
                {
                    throw Failure("Scene UUIDs and addresses must be nonempty.");
                }

                if (!desired.TryAdd(id, scene))
                {
                    throw Failure($"Scene UUID {id} appeared more than once.");
                }

                if (!addresses.Add(address))
                {
                    throw Failure($"Scene address '{address}' appeared more than once.");
                }
            }

            Guid? primaryId = primarySceneId?.Value;
            if (primaryId is null && scenes.Count == 1)
            {
                primaryId = scenes[0].Id.Value;
            }

            if (scenes.Count > 1 && primaryId is null)
            {
                throw Failure("A snapshot with multiple scenes must select a primary scene.");
            }

            if (primaryId is Guid selected && !desired.ContainsKey(selected))
            {
                throw Failure($"Primary scene {selected} is not in the snapshot.");
            }

            return new Replacement(desired, scenes.ToArray(), primaryId);
        }

        private void StartPendingLoads()
        {
            if (pending!.LoadsStarted)
            {
                return;
            }

            pending.LoadsStarted = true;
            foreach (MasonryScene scene in pending.Ordered)
            {
                Guid id = scene.Id.Value;
                if (pending.Retained.ContainsKey(id))
                {
                    continue;
                }

                var asset = new PreparedAsset.Scene(scene.Address);
                IMasonryAssetLease lease = preparedAssets.Acquire(asset);
                try
                {
                    pending.Loading.Add(new Entry(id, scene.Address, storage.LoadScene(lease)));
                }
                catch
                {
                    lease.Dispose();
                    throw;
                }
            }
        }

        private void BeginUnload(Entry entry)
        {
            world.RemoveScene(new SceneId(entry.Id));
            entry.Handle.BeginUnload();
            unloading.Add(entry);
        }

        private void CancelPendingLoads()
        {
            if (pending is null)
            {
                return;
            }

            foreach (Entry entry in pending.Loading)
            {
                entry.Handle.BeginUnload();
                entry.Handle.Dispose();
            }

            pending = null;
        }

        private static MasonryAssetException Failure(
            string message,
            Exception? innerException = null
        ) => new(CoreErrorCode.UnknownScene, message, innerException);

        private void ThrowIfDisposed()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryScenes));
            }
        }

        private sealed record Entry(Guid Id, SceneAddress Address, IMasonrySceneHandle Handle);

        private sealed class Replacement
        {
            public Replacement(
                Dictionary<Guid, MasonryScene> desired,
                IReadOnlyList<MasonryScene> ordered,
                Guid? primaryId
            )
            {
                Desired = desired;
                Ordered = ordered;
                PrimaryId = primaryId;
            }

            public Dictionary<Guid, MasonryScene> Desired { get; }

            public IReadOnlyList<MasonryScene> Ordered { get; }

            public Guid? PrimaryId { get; }

            public Dictionary<Guid, Entry> Retained { get; } = new();

            public List<Entry> Loading { get; } = new();

            public bool LoadsStarted { get; set; }
        }
    }
}
