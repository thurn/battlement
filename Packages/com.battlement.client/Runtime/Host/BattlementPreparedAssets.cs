#nullable enable

using System;
using System.Collections.Generic;
using System.Text;
using UnityEngine;

namespace Battlement
{
    /// <summary>Looks up assets from the current prepared set without loading them.</summary>
    public interface IBattlementPreparedAssetLookup
    {
        /// <summary>Returns the prepared value for an exact declaration when available.</summary>
        bool TryGet(PreparedAsset asset, out object? value);
    }

    /// <summary>Owns the active prepared set and its protocol-level usage leases.</summary>
    internal sealed class BattlementPreparedAssets
        : IDisposable,
            IBattlementPreparedAssetLookup,
            Battlement.UI.IBattlementUiAssetLookup
    {
        private const int MaximumAssets = 16_384;
        private const int MaximumStringBytes = 65_536;

        private readonly IBattlementAssetStorage storage;
        private readonly List<Entry> retired = new();
        private Dictionary<string, Entry> active = new(StringComparer.Ordinal);
        private Replacement? pending;
        private bool isDisposed;

        public BattlementPreparedAssets(IBattlementAssetStorage storage) => this.storage = storage;

        /// <summary>
        /// Validates a complete replacement and starts loading its new entries.
        /// </summary>
        /// <param name="assets">The complete set that should become active atomically.</param>
        /// <param name="isAuthoritative">
        /// Whether a snapshot may retire in-use entries. Pass <c>false</c> for a command,
        /// which must reject removal of an entry that still has a usage lease.
        /// </param>
        public void BeginReplacement(IReadOnlyList<PreparedAsset> assets, bool isAuthoritative)
        {
            ThrowIfDisposed();
            Preconditions.CheckNotNull(assets, nameof(assets));
            CancelPending();

            if (assets.Count > MaximumAssets)
            {
                throw Failure(
                    CoreErrorCode.LimitExceeded,
                    $"A prepared set cannot contain more than {MaximumAssets} assets."
                );
            }

            var declarations = new List<(string Address, PreparedAsset Asset)>(assets.Count);
            var validated = new Dictionary<string, PreparedAsset>(
                assets.Count,
                StringComparer.Ordinal
            );
            foreach (PreparedAsset asset in assets)
            {
                string address = AddressOf(asset);
                if (Encoding.UTF8.GetByteCount(address) > MaximumStringBytes)
                {
                    throw Failure(
                        CoreErrorCode.LimitExceeded,
                        $"A prepared asset address exceeds {MaximumStringBytes} bytes."
                    );
                }

                if (!validated.TryAdd(address, asset))
                {
                    throw Failure(
                        CoreErrorCode.DuplicateId,
                        $"Prepared asset address '{address}' appeared more than once."
                    );
                }

                declarations.Add((address, asset));
            }

            if (!isAuthoritative)
            {
                foreach ((string address, Entry entry) in active)
                {
                    if (
                        (
                            !validated.TryGetValue(address, out PreparedAsset replacement)
                            || replacement != entry.Asset
                        )
                        && entry.UsageCount > 0
                    )
                    {
                        throw Failure(
                            CoreErrorCode.AssetInUse,
                            $"Prepared asset '{address}' is still in use."
                        );
                    }
                }
            }

            var target = new Dictionary<string, Entry>(assets.Count, StringComparer.Ordinal);
            var additions = new List<Entry>();
            try
            {
                foreach ((string address, PreparedAsset asset) in declarations)
                {
                    if (active.TryGetValue(address, out Entry existing) && existing.Asset == asset)
                    {
                        target.Add(address, existing);
                    }
                    else
                    {
                        var addition = new Entry(asset, storage.Prepare(asset));
                        additions.Add(addition);
                        target.Add(address, addition);
                    }
                }

                pending = new Replacement(target, additions, isAuthoritative);
            }
            catch
            {
                foreach (Entry addition in additions)
                {
                    addition.Dispose();
                }

                throw;
            }
        }

        /// <summary>
        /// Polls pending loads and commits the replacement once every addition succeeds.
        /// </summary>
        /// <param name="error">The stable failure if preparation finishes unsuccessfully.</param>
        /// <returns><c>false</c> while at least one load is still pending.</returns>
        public bool TryCompleteReplacement(out BattlementAssetException? error)
        {
            ThrowIfDisposed();
            error = null;
            if (pending is null)
            {
                return true;
            }

            try
            {
                foreach (Entry addition in pending.Additions)
                {
                    if (!addition.Handle.IsDone)
                    {
                        return false;
                    }

                    if (addition.Handle.Error is Exception failure)
                    {
                        throw failure;
                    }

                    if (addition.Handle.Value is null)
                    {
                        throw Failure(
                            CoreErrorCode.UnknownAsset,
                            $"Prepared asset '{AddressOf(addition.Asset)}' resolved to no value."
                        );
                    }

                    ValidatePreparedValue(addition);
                }

                CommitPending();
                return true;
            }
            catch (Exception exception)
            {
                error =
                    exception as BattlementAssetException
                    ?? Failure(
                        CoreErrorCode.UnknownAsset,
                        $"Asset preparation failed: {exception.Message}",
                        exception
                    );
                CancelPending();
                return true;
            }
        }

        /// <summary>Looks up an active entry without loading or acquiring a lease.</summary>
        public bool TryGet(PreparedAsset asset, out object? value)
        {
            ThrowIfDisposed();
            if (active.TryGetValue(AddressOf(asset), out Entry entry) && entry.Asset == asset)
            {
                value = entry.Handle.Value;
                return value is not null;
            }

            value = null;
            return false;
        }

        /// <summary>
        /// Acquires a lease that prevents command-driven removal until it is disposed.
        /// </summary>
        public IBattlementAssetLease Acquire(PreparedAsset asset)
        {
            ThrowIfDisposed();
            if (
                !active.TryGetValue(AddressOf(asset), out Entry entry)
                || entry.Asset != asset
                || entry.Handle.Value is not object value
            )
            {
                throw Failure(
                    CoreErrorCode.AssetNotPrepared,
                    $"Asset '{AddressOf(asset)}' was not in the prepared set."
                );
            }

            entry.UsageCount++;
            return new Lease(this, entry, value);
        }

        Battlement.UI.IBattlementUiAssetLease Battlement.UI.IBattlementUiAssetLookup.Acquire(
            PreparedAsset asset
        ) => Acquire(asset);

        /// <summary>Abandons an uncommitted replacement and releases its new handles.</summary>
        public void CancelPending()
        {
            if (pending is null)
            {
                return;
            }

            foreach (Entry addition in pending.Additions)
            {
                addition.Dispose();
            }

            pending = null;
        }

        /// <summary>Releases the complete prepared set while retaining this owner.</summary>
        public void BeginSession()
        {
            ThrowIfDisposed();
            CancelPending();
            foreach (Entry entry in active.Values)
            {
                if (entry.UsageCount > 0)
                {
                    retired.Add(entry);
                }
                else
                {
                    entry.Dispose();
                }
            }
            active.Clear();
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            CancelPending();
            foreach (Entry entry in active.Values)
            {
                entry.Dispose();
            }

            foreach (Entry entry in retired)
            {
                entry.Dispose();
            }

            active.Clear();
            retired.Clear();
            isDisposed = true;
        }

        private void CommitPending()
        {
            Replacement replacement = pending!;
            foreach ((string address, Entry entry) in active)
            {
                if (
                    replacement.Target.TryGetValue(address, out Entry retained)
                    && retained == entry
                )
                {
                    continue;
                }

                if (replacement.IsAuthoritative && entry.UsageCount > 0)
                {
                    retired.Add(entry);
                }
                else
                {
                    entry.Dispose();
                }
            }

            active = replacement.Target;
            pending = null;
        }

        private void Release(Entry entry)
        {
            entry.UsageCount--;
            if (entry.UsageCount == 0 && retired.Remove(entry))
            {
                entry.Dispose();
            }
        }

        private static string AddressOf(PreparedAsset asset) =>
            Preconditions.CheckNotNull(asset, nameof(asset)) switch
            {
                PreparedAsset.Scene value => RequireAddress(value.Address.Value),
                PreparedAsset.Prefab value => RequireAddress(value.Address.Value),
                PreparedAsset.ParticleEffect value => RequireAddress(value.Address.Value),
                PreparedAsset.Material value => RequireAddress(value.Address.Value),
                PreparedAsset.Texture value => RequireAddress(value.Address.Value),
                PreparedAsset.Sprite value => RequireAddress(value.Address.Value),
                PreparedAsset.VectorImage value => RequireAddress(value.Address.Value),
                PreparedAsset.RenderTexture value => RequireAddress(value.Address.Value),
                PreparedAsset.AudioClip value => RequireAddress(value.Address.Value),
                PreparedAsset.TextMeshProFont value => RequireAddress(value.Address.Value),
                PreparedAsset.UiFont value => RequireAddress(value.Address.Value),
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown prepared asset kind."),
            };

        private static void ValidatePreparedValue(Entry entry)
        {
            if (
                entry.Asset is not PreparedAsset.Prefab
                && entry.Asset is not PreparedAsset.ParticleEffect
            )
            {
                return;
            }

            if (entry.Handle.Value is not GameObject prefab)
            {
                throw Failure(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared object asset '{AddressOf(entry.Asset)}' did not resolve "
                        + "as a GameObject."
                );
            }

            if (entry.Asset is PreparedAsset.ParticleEffect)
            {
                if (prefab.GetComponentsInChildren<ParticleSystem>(true).Length == 0)
                {
                    throw Failure(
                        CoreErrorCode.ComponentMissing,
                        $"Particle effect '{AddressOf(entry.Asset)}' has no ParticleSystem."
                    );
                }

                return;
            }

            ValidateSingleRootComponent<Renderer>(prefab, "Renderer");
            ValidateSingleRootComponent<Animator>(prefab, "Animator");
            ValidateSingleRootComponent<Camera>(prefab, "Camera");
            ValidateSingleRootComponent<Light>(prefab, "Light");
        }

        private static void ValidateSingleRootComponent<T>(GameObject prefab, string componentName)
            where T : Component
        {
            if (prefab.GetComponents<T>().Length > 1)
            {
                throw Failure(
                    CoreErrorCode.InvalidComponentCount,
                    $"Prefab '{prefab.name}' has more than one root {componentName}."
                );
            }
        }

        private static string RequireAddress(string? address) =>
            !string.IsNullOrEmpty(address)
                ? address
                : throw Failure(CoreErrorCode.UnknownAsset, "An asset address cannot be empty.");

        private static BattlementAssetException Failure(
            CoreErrorCode errorCode,
            string message,
            Exception? innerException = null
        ) => new(errorCode, message, innerException);

        private void ThrowIfDisposed()
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(BattlementPreparedAssets));
            }
        }

        private sealed class Entry : IDisposable
        {
            public Entry(PreparedAsset asset, IBattlementAssetHandle handle)
            {
                Asset = asset;
                Handle = handle;
            }

            public PreparedAsset Asset { get; }

            public IBattlementAssetHandle Handle { get; }

            public int UsageCount { get; set; }

            public void Dispose() => Handle.Dispose();
        }

        private sealed record Replacement(
            Dictionary<string, Entry> Target,
            List<Entry> Additions,
            bool IsAuthoritative
        );

        private sealed class Lease : IBattlementAssetLease
        {
            private readonly BattlementPreparedAssets owner;
            private Entry? entry;

            public Lease(BattlementPreparedAssets owner, Entry entry, object value)
            {
                this.owner = owner;
                this.entry = entry;
                Value = value;
            }

            public PreparedAsset Asset =>
                entry?.Asset ?? throw new ObjectDisposedException(nameof(Lease));

            public object Value { get; }

            public void Dispose()
            {
                if (entry is not Entry retained)
                {
                    return;
                }

                entry = null;
                owner.Release(retained);
            }
        }
    }
}
