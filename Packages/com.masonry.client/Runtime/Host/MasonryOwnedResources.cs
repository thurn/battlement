#nullable enable

using UnityEngine;

namespace Masonry
{
    /// <summary>Releases a resource retained by one Masonry-owned object.</summary>
    internal interface IMasonryOwnedResource
    {
        void Release();
    }

    /// <summary>Centralizes eager resource release before Unity destroys an object.</summary>
    internal static class MasonryOwnedResources
    {
        public static void Release(GameObject gameObject)
        {
            foreach (MonoBehaviour component in gameObject.GetComponents<MonoBehaviour>())
            {
                if (component is IMasonryOwnedResource resource)
                {
                    resource.Release();
                }
            }
        }
    }
}
