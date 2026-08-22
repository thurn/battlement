#nullable enable

using UnityEngine;

namespace Battlement
{
    /// <summary>Releases a resource retained by one Battlement-owned object.</summary>
    internal interface IBattlementOwnedResource
    {
        void Release();
    }

    /// <summary>Centralizes eager resource release before Unity destroys an object.</summary>
    internal static class BattlementOwnedResources
    {
        public static void Release(GameObject gameObject)
        {
            foreach (MonoBehaviour component in gameObject.GetComponents<MonoBehaviour>())
            {
                if (component is IBattlementOwnedResource resource)
                {
                    resource.Release();
                }
            }
        }
    }
}
