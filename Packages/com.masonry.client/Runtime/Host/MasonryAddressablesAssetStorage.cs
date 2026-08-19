#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using TMPro;
using UnityEngine;
using UnityEngine.AddressableAssets;
using UnityEngine.ResourceManagement.AsyncOperations;
using UnityEngine.ResourceManagement.ResourceLocations;
using UnityEngine.ResourceManagement.ResourceProviders;

namespace Masonry
{
    /// <summary>Prepares Masonry assets through Unity Addressables.</summary>
    public sealed class MasonryAddressablesAssetStorage : IMasonryAssetStorage
    {
        private readonly HashSet<IMasonryAssetHandle> handles = new();
        private bool isDisposed;

        /// <inheritdoc />
        public IMasonryAssetHandle Prepare(PreparedAsset asset)
        {
            if (isDisposed)
            {
                throw new ObjectDisposedException(nameof(MasonryAddressablesAssetStorage));
            }

            IMasonryAssetHandle handle = Errors.CheckNotNull(asset, nameof(asset)) switch
            {
                PreparedAsset.Scene value => new SceneHandle(value, Remove),
                PreparedAsset.Prefab value => new AssetHandle<GameObject>(value, Remove),
                PreparedAsset.ParticleEffect value => new AssetHandle<GameObject>(value, Remove),
                PreparedAsset.Material value => new AssetHandle<Material>(value, Remove),
                PreparedAsset.Texture value => new AssetHandle<Texture>(value, Remove),
                PreparedAsset.AudioClip value => new AssetHandle<AudioClip>(value, Remove),
                PreparedAsset.Font value => new AssetHandle<TMP_FontAsset>(value, Remove),
                _ => throw new MasonryAssetException(
                    CoreErrorCode.UnknownAsset,
                    "Unknown prepared asset kind."
                ),
            };
            handles.Add(handle);
            return handle;
        }

        /// <inheritdoc />
        public void Dispose()
        {
            foreach (IMasonryAssetHandle handle in handles.ToArray())
            {
                handle.Dispose();
            }

            handles.Clear();
            isDisposed = true;
        }

        private void Remove(IMasonryAssetHandle handle) => handles.Remove(handle);

        private static string AddressOf(PreparedAsset asset) =>
            asset switch
            {
                PreparedAsset.Scene value => value.Address.Value,
                PreparedAsset.Prefab value => value.Address.Value,
                PreparedAsset.ParticleEffect value => value.Address.Value,
                PreparedAsset.Material value => value.Address.Value,
                PreparedAsset.Texture value => value.Address.Value,
                PreparedAsset.AudioClip value => value.Address.Value,
                PreparedAsset.Font value => value.Address.Value,
                _ => throw new MasonryAssetException(
                    CoreErrorCode.UnknownAsset,
                    "Unknown prepared asset kind."
                ),
            };

        private sealed class AssetHandle<T> : IMasonryAssetHandle
            where T : UnityEngine.Object
        {
            private readonly Action<IMasonryAssetHandle> onDispose;
            private AsyncOperationHandle<IList<IResourceLocation>> locations;
            private AsyncOperationHandle<T>? load;
            private Exception? error;
            private bool locationsReleased;
            private bool isDisposed;

            public AssetHandle(PreparedAsset asset, Action<IMasonryAssetHandle> onDispose)
            {
                Asset = asset;
                this.onDispose = onDispose;
                locations = Addressables.LoadResourceLocationsAsync(AddressOf(asset));
            }

            public PreparedAsset Asset { get; }

            public bool IsDone
            {
                get
                {
                    Advance();
                    return error is not null || (load is not null && load.Value.IsDone);
                }
            }

            public object? Value
            {
                get
                {
                    Advance();
                    return load is { IsDone: true, Status: AsyncOperationStatus.Succeeded }
                        ? load.Value.Result
                        : null;
                }
            }

            public Exception? Error
            {
                get
                {
                    Advance();
                    if (
                        error is null
                        && load is { IsDone: true, Status: AsyncOperationStatus.Failed }
                    )
                    {
                        error = new MasonryAssetException(
                            CoreErrorCode.UnknownAsset,
                            $"Addressables failed to load '{AddressOf(Asset)}'.",
                            load.Value.OperationException
                        );
                    }

                    return error;
                }
            }

            public void Dispose()
            {
                if (isDisposed)
                {
                    return;
                }

                if (load is AsyncOperationHandle<T> loadHandle)
                {
                    Addressables.Release(loadHandle);
                }

                ReleaseLocations();
                isDisposed = true;
                onDispose(this);
            }

            private void Advance()
            {
                if (isDisposed || error is not null || load is not null || !locations.IsDone)
                {
                    return;
                }

                if (locations.Status == AsyncOperationStatus.Failed)
                {
                    error = new MasonryAssetException(
                        CoreErrorCode.UnknownAsset,
                        $"Addressables could not resolve '{AddressOf(Asset)}'.",
                        locations.OperationException
                    );
                    return;
                }

                IResourceLocation? location = locations.Result.FirstOrDefault(candidate =>
                    typeof(T).IsAssignableFrom(candidate.ResourceType)
                );
                if (location is null)
                {
                    error = new MasonryAssetException(
                        locations.Result.Count == 0
                            ? CoreErrorCode.UnknownAsset
                            : CoreErrorCode.AssetTypeMismatch,
                        $"Addressable '{AddressOf(Asset)}' did not resolve as {typeof(T).Name}."
                    );
                    return;
                }

                load = Addressables.LoadAssetAsync<T>(location);
                ReleaseLocations();
            }

            private void ReleaseLocations()
            {
                if (!locationsReleased)
                {
                    Addressables.Release(locations);
                    locationsReleased = true;
                }
            }
        }

        private sealed class SceneHandle : IMasonryAssetHandle
        {
            private readonly Action<IMasonryAssetHandle> onDispose;
            private AsyncOperationHandle<IList<IResourceLocation>> locations;
            private AsyncOperationHandle? download;
            private IResourceLocation? location;
            private Exception? error;
            private bool locationsReleased;
            private bool isDisposed;

            public SceneHandle(PreparedAsset.Scene asset, Action<IMasonryAssetHandle> onDispose)
            {
                Asset = asset;
                this.onDispose = onDispose;
                locations = Addressables.LoadResourceLocationsAsync(asset.Address.Value);
            }

            public PreparedAsset Asset { get; }

            public bool IsDone
            {
                get
                {
                    Advance();
                    return error is not null || (download is not null && download.Value.IsDone);
                }
            }

            public object? Value
            {
                get
                {
                    Advance();
                    return download is { IsDone: true, Status: AsyncOperationStatus.Succeeded }
                        ? location
                        : null;
                }
            }

            public Exception? Error
            {
                get
                {
                    Advance();
                    if (
                        error is null
                        && download is { IsDone: true, Status: AsyncOperationStatus.Failed }
                    )
                    {
                        error = new MasonryAssetException(
                            CoreErrorCode.UnknownAsset,
                            $"Addressables failed to download '{AddressOf(Asset)}'.",
                            download.Value.OperationException
                        );
                    }

                    return error;
                }
            }

            public void Dispose()
            {
                if (isDisposed)
                {
                    return;
                }

                if (download is AsyncOperationHandle downloadHandle)
                {
                    Addressables.Release(downloadHandle);
                }

                ReleaseLocations();
                isDisposed = true;
                onDispose(this);
            }

            private void Advance()
            {
                if (isDisposed || error is not null || download is not null || !locations.IsDone)
                {
                    return;
                }

                if (locations.Status == AsyncOperationStatus.Failed)
                {
                    error = new MasonryAssetException(
                        CoreErrorCode.UnknownAsset,
                        $"Addressables could not resolve scene '{AddressOf(Asset)}'.",
                        locations.OperationException
                    );
                    return;
                }

                location = locations.Result.FirstOrDefault(candidate =>
                    typeof(SceneInstance).IsAssignableFrom(candidate.ResourceType)
                );
                if (location is null)
                {
                    error = new MasonryAssetException(
                        locations.Result.Count == 0
                            ? CoreErrorCode.UnknownAsset
                            : CoreErrorCode.AssetTypeMismatch,
                        $"Addressable '{AddressOf(Asset)}' did not resolve as a scene."
                    );
                    return;
                }

                download = Addressables.DownloadDependenciesAsync(location, false);
                ReleaseLocations();
            }

            private void ReleaseLocations()
            {
                if (!locationsReleased)
                {
                    Addressables.Release(locations);
                    locationsReleased = true;
                }
            }
        }
    }
}
