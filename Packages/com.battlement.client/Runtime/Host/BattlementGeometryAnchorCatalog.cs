#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Battlement
{
    internal interface IBattlementGeometryAnchorLease
    {
        BattlementGeometryAnchorCatalog GeometryAnchors { get; }
    }

    internal sealed class BattlementGeometryAnchorCatalog
    {
        public static readonly BattlementGeometryAnchorCatalog Empty = new(
            new Dictionary<string, int[]>(StringComparer.Ordinal)
        );

        private readonly IReadOnlyDictionary<string, int[]> paths;

        private BattlementGeometryAnchorCatalog(IReadOnlyDictionary<string, int[]> paths) =>
            this.paths = paths;

        public static BattlementGeometryAnchorCatalog Capture(GameObject root)
        {
            var paths = new Dictionary<string, int[]>(StringComparer.Ordinal);
            BattlementGeometryAnchor[] anchors =
                root.GetComponentsInChildren<BattlementGeometryAnchor>(true);
            foreach (BattlementGeometryAnchor anchor in anchors)
            {
                if (string.IsNullOrEmpty(anchor.Name))
                    throw new InvalidOperationException(
                        $"Prefab '{root.name}' has an empty geometry anchor name."
                    );
                if (!paths.TryAdd(anchor.Name, Path(root.transform, anchor.transform)))
                    throw new InvalidOperationException(
                        $"Prefab '{root.name}' has duplicate geometry anchor '{anchor.Name}'."
                    );
            }
            return new BattlementGeometryAnchorCatalog(paths);
        }

        public Transform Resolve(GameObject root, AnchorName name)
        {
            if (!paths.TryGetValue(name.Value, out int[] path))
                throw Missing(root, name);

            Transform current = root.transform;
            foreach (int siblingIndex in path)
            {
                if (siblingIndex >= current.childCount)
                    throw Missing(root, name);
                current = current.GetChild(siblingIndex);
            }
            return current;
        }

        private static int[] Path(Transform root, Transform target)
        {
            var reversed = new List<int>();
            Transform? current = target;
            while (current != root)
            {
                if (current == null)
                    throw new InvalidOperationException(
                        "A geometry anchor must belong to its prefab."
                    );
                reversed.Add(current.GetSiblingIndex());
                current = current.parent;
            }
            reversed.Reverse();
            return reversed.ToArray();
        }

        private static InvalidOperationException Missing(GameObject root, AnchorName name) =>
            new($"Object '{root.name}' has no prepared geometry anchor named '{name.Value}'.");
    }
}
