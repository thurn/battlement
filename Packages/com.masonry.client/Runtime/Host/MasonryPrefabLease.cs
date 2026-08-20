#nullable enable

using UnityEngine;

namespace Masonry
{
    /// <summary>
    /// Retains one prepared prefab's protocol usage lease for an instance's lifetime.
    /// </summary>
    /// <remarks>
    /// The lease contributes to Masonry's prepared-asset usage count. It prevents a
    /// command-driven prepared-set replacement from removing the prefab while this
    /// instance exists; Addressables retains the underlying asset through its load handle.
    /// </remarks>
    [DisallowMultipleComponent]
    internal sealed class MasonryPrefabLease : MonoBehaviour, IMasonryOwnedResource
    {
        private IMasonryAssetLease? lease;

        /// <summary>Associates the instance with its acquired prepared-prefab lease.</summary>
        internal void Initialize(IMasonryAssetLease value) => lease = value;

        /// <summary>Releases the instance's contribution to the prefab usage count.</summary>
        internal void Release()
        {
            lease?.Dispose();
            lease = null;
        }

        void IMasonryOwnedResource.Release() => Release();

        private void OnDestroy() => Release();
    }
}
