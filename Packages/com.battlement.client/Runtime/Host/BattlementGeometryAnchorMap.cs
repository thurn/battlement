#nullable enable

using UnityEngine;

namespace Battlement
{
    [DisallowMultipleComponent]
    internal sealed class BattlementGeometryAnchorMap : MonoBehaviour
    {
        private BattlementGeometryAnchorCatalog catalog = BattlementGeometryAnchorCatalog.Empty;

        public static void Attach(GameObject root, BattlementGeometryAnchorCatalog value) =>
            root.AddComponent<BattlementGeometryAnchorMap>().catalog = value;

        public Transform Resolve(AnchorName name) => catalog.Resolve(gameObject, name);
    }
}
