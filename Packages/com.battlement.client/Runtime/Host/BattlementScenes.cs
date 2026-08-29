#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement
{
    internal sealed class BattlementScenes : IDisposable
    {
        private const int MaximumScenes = 32;

        private readonly IBattlementAssetStorage storage;
        private readonly BattlementPreparedAssets preparedAssets;
        private readonly BattlementWorld world;
        private readonly Dictionary<Guid, Entry> loaded = new();
        private readonly List<Entry> unloading = new();
        private Replacement? pending;
        private Guid? primaryId;
        private BattlementAssetException? sessionResetError;
        private bool isDisposed;

        public BattlementScenes(
            IBattlementAssetStorage storage,
            BattlementPreparedAssets preparedAssets,
            BattlementWorld world
        )
        {
            this.storage = storage;
            this.preparedAssets = preparedAssets;
            this.world = world;
        }

        public void BeginSession()
        {
            ThrowIfDisposed();
            sessionResetError = null;
            CancelPendingLoads();
            foreach (Entry entry in loaded.Values.ToArray())
            {
                BeginUnload(entry);
            }

            loaded.Clear();
            primaryId = null;
        }

        public bool TryCompleteSessionReset(out BattlementAssetException? error)
        {
            ThrowIfDisposed();
            bool isPending = false;
            foreach (Entry entry in unloading.ToArray())
            {
                if (entry.Handle.Error is Exception unloadError)
                {
                    sessionResetError ??= Failure(
                        $"Scene '{entry.Address.Value}' failed to unload: {unloadError.Message}",
                        unloadError
                    );
                    unloading.Remove(entry);
                    entry.Handle.Dispose();
                    continue;
                }

                if (!entry.Handle.IsUnloaded)
                {
                    isPending = true;
                    continue;
                }

                unloading.Remove(entry);
                entry.Handle.Dispose();
            }

            error = sessionResetError;
            return !isPending;
        }

        public void BeginLoad(SceneId sceneId, SceneAddress address, bool makePrimary)
        {
            ThrowIfDisposed();
            Guid id = sceneId.Value;
            if (id == Guid.Empty || string.IsNullOrEmpty(address.Value))
            {
                throw Failure("Scene UUIDs and addresses must be nonempty.");
            }

            if (loaded.ContainsKey(id))
            {
                throw new BattlementAssetException(
                    CoreErrorCode.DuplicateId,
                    $"Scene UUID {id} was already loaded."
                );
            }

            if (loaded.Values.Any(entry => entry.Address == address))
            {
                throw new BattlementAssetException(
                    CoreErrorCode.DuplicateId,
                    $"Scene address '{address.Value}' is already loaded."
                );
            }

            BattlementScene[] desired = loaded
                .Values.Select(entry => new BattlementScene(new SceneId(entry.Id), entry.Address))
                .Append(new BattlementScene(sceneId, address))
                .ToArray();
            BeginReplacement(desired, makePrimary ? sceneId : PrimarySceneId());
        }

        public void ValidateUnload(SceneId sceneId)
        {
            Guid id = sceneId.Value;
            if (!loaded.ContainsKey(id))
            {
                throw Failure($"Scene {id} does not exist.");
            }

            if (primaryId == id)
            {
                throw new BattlementAssetException(
                    CoreErrorCode.InvalidProperty,
                    $"Primary scene {id} cannot be unloaded."
                );
            }
        }

        public void BeginUnload(SceneId sceneId)
        {
            ValidateUnload(sceneId);
            BattlementScene[] desired = loaded
                .Values.Where(entry => entry.Id != sceneId.Value)
                .Select(entry => new BattlementScene(new SceneId(entry.Id), entry.Address))
                .ToArray();
            BeginReplacement(desired, PrimarySceneId());
        }

        public IBattlementCommandOperation? SetPrimary(SceneId sceneId)
        {
            if (!loaded.ContainsKey(sceneId.Value))
            {
                throw Failure($"Scene {sceneId.Value} does not exist.");
            }

            world.SetPrimaryScene(sceneId);
            primaryId = sceneId.Value;
            return null;
        }

        public void CancelPendingCommand() => CancelPendingLoads();

        public void BeginReplacement(IReadOnlyList<BattlementScene> scenes, SceneId? primarySceneId)
        {
            ThrowIfDisposed();
            Preconditions.CheckNotNull(scenes, nameof(scenes));
            if (pending is not null)
            {
                throw Failure("A scene replacement is already in progress.");
            }

            Replacement replacement = Validate(scenes, primarySceneId);
            foreach (Entry entry in loaded.Values.ToArray())
            {
                if (
                    replacement.Desired.TryGetValue(entry.Id, out BattlementScene desired)
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

        public bool TryCompleteReplacement(out BattlementAssetException? error)
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
                    this.primaryId = primaryId;
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
                error = MapFailure(exception);
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
            primaryId = null;
            isDisposed = true;
        }

        private SceneId? PrimarySceneId() => primaryId is Guid value ? new SceneId(value) : null;

        private Replacement Validate(IReadOnlyList<BattlementScene> scenes, SceneId? primarySceneId)
        {
            if (scenes.Count > MaximumScenes)
            {
                throw new BattlementAssetException(
                    CoreErrorCode.LimitExceeded,
                    $"Battlement cannot load more than {MaximumScenes} content scenes."
                );
            }

            var desired = new Dictionary<Guid, BattlementScene>();
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            foreach (BattlementScene scene in scenes)
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
            foreach (BattlementScene scene in pending.Ordered)
            {
                Guid id = scene.Id.Value;
                if (pending.Retained.ContainsKey(id))
                {
                    continue;
                }

                var asset = new PreparedAsset.Scene(scene.Address);
                IBattlementAssetLease lease = preparedAssets.Acquire(asset);
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

        private static BattlementAssetException Failure(
            string message,
            Exception? innerException = null
        ) => new(CoreErrorCode.UnknownScene, message, innerException);

        private static BattlementAssetException MapFailure(Exception exception) =>
            exception switch
            {
                BattlementAssetException assetError => assetError,
                BattlementWorldException worldError => new BattlementAssetException(
                    worldError.ErrorCode,
                    worldError.Message,
                    worldError
                ),
                _ => Failure($"Scene replacement failed: {exception.Message}", exception),
            };

        private void ThrowIfDisposed()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementScenes));
            }
        }

        private sealed record Entry(Guid Id, SceneAddress Address, IBattlementSceneHandle Handle);

        private sealed class Replacement
        {
            public Replacement(
                Dictionary<Guid, BattlementScene> desired,
                IReadOnlyList<BattlementScene> ordered,
                Guid? primaryId
            )
            {
                Desired = desired;
                Ordered = ordered;
                PrimaryId = primaryId;
            }

            public Dictionary<Guid, BattlementScene> Desired { get; }

            public IReadOnlyList<BattlementScene> Ordered { get; }

            public Guid? PrimaryId { get; }

            public Dictionary<Guid, Entry> Retained { get; } = new();

            public List<Entry> Loading { get; } = new();

            public bool LoadsStarted { get; set; }
        }
    }
}
