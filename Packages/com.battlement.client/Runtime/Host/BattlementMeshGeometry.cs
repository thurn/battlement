#nullable enable

using System;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement
{
    /// <summary>Instantiates prepared geometry without changing its authored coordinates.</summary>
    internal static class BattlementMeshGeometry
    {
        internal static (GameObject, IBattlementAssetLease?) Create(
            BattlementPreparedAssets assets,
            MeshAddress address,
            bool pointerEvents,
            Action<GameObject> applyMaterials
        )
        {
            IBattlementAssetLease lease = assets.Acquire(new PreparedAsset.Mesh(address));
            GameObject? instance = null;
            try
            {
                if (lease.Value is not Mesh mesh)
                    throw new BattlementWorldException(
                        CoreErrorCode.AssetTypeMismatch,
                        $"Prepared mesh '{address}' is not a Mesh."
                    );
                if (mesh.subMeshCount == 0)
                    throw new BattlementWorldException(
                        CoreErrorCode.InvalidProperty,
                        "A prepared mesh must have submeshes."
                    );
                instance = new GameObject("Battlement Mesh");
                instance.AddComponent<MeshFilter>().sharedMesh = mesh;
                instance.AddComponent<MeshRenderer>().sharedMaterials = new Material[
                    mesh.subMeshCount
                ];
                applyMaterials(instance);
                if (pointerEvents)
                    instance.AddComponent<MeshCollider>().sharedMesh = mesh;
                return (instance, lease);
            }
            catch
            {
                lease.Dispose();
                if (instance != null)
                {
                    var resources = instance.GetComponents<IBattlementOwnedResource>();
                    foreach (IBattlementOwnedResource resource in resources)
                        resource.Release();
                    if (Application.isPlaying)
                        Object.Destroy(instance);
                    else
                        Object.DestroyImmediate(instance);
                }
                throw;
            }
        }
    }
}
