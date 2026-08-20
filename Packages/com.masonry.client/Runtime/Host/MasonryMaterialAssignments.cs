#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Masonry
{
    [DisallowMultipleComponent]
    internal sealed class MasonryMaterialAssignments : MonoBehaviour
    {
        private readonly Dictionary<string, MaterialLease> leases = new(StringComparer.Ordinal);
        private readonly Dictionary<int, MaterialLease> slots = new();
        private MasonryPreparedAssets? preparedAssets;
        private Renderer? targetRenderer;

        internal void Initialize(
            Renderer renderer,
            MasonryPreparedAssets assets,
            IReadOnlyList<MaterialAssignment> assignments
        )
        {
            targetRenderer = renderer;
            preparedAssets = assets;
            var uniqueSlots = new HashSet<uint>();
            foreach (MaterialAssignment assignment in assignments)
            {
                if (!uniqueSlots.Add(assignment.Slot))
                {
                    throw Invalid($"Renderer material slot {assignment.Slot} appeared twice.");
                }
            }

            foreach (MaterialAssignment assignment in assignments)
            {
                SetMaterial(assignment.Address, assignment.Slot);
            }
        }

        internal void SetMaterial(MaterialAddress address, uint? slot)
        {
            if (targetRenderer == null || preparedAssets == null)
            {
                throw new InvalidOperationException("Material assignments are not initialized.");
            }

            Material[] materials = targetRenderer.sharedMaterials;
            if (materials.Length == 0)
            {
                throw new MasonryWorldException(
                    CoreErrorCode.ComponentMissing,
                    "The root Renderer has no material slots."
                );
            }

            int firstSlot;
            int slotCount;
            if (slot is uint requestedSlot)
            {
                if (requestedSlot >= materials.Length)
                {
                    throw Invalid(
                        $"Renderer material slot {requestedSlot} is outside the "
                            + $"available range [0, {materials.Length - 1}]."
                    );
                }

                firstSlot = checked((int)requestedSlot);
                slotCount = 1;
            }
            else
            {
                firstSlot = 0;
                slotCount = materials.Length;
            }

            MaterialLease replacement = Acquire(address);
            try
            {
                for (int index = firstSlot; index < firstSlot + slotCount; index++)
                {
                    materials[index] = replacement.Material;
                }

                targetRenderer.sharedMaterials = materials;
            }
            catch
            {
                ReleaseUnused(replacement);
                throw;
            }

            for (int index = firstSlot; index < firstSlot + slotCount; index++)
            {
                if (slots.TryGetValue(index, out MaterialLease previous))
                {
                    if (ReferenceEquals(previous, replacement))
                    {
                        continue;
                    }

                    previous.SlotCount--;
                    ReleaseUnused(previous);
                }

                slots[index] = replacement;
                replacement.SlotCount++;
            }
        }

        internal void Release()
        {
            slots.Clear();
            foreach (MaterialLease lease in leases.Values)
            {
                lease.Lease.Dispose();
            }

            leases.Clear();
        }

        private MaterialLease Acquire(MaterialAddress address)
        {
            if (string.IsNullOrEmpty(address.Value))
            {
                throw Invalid("A material address cannot be empty.");
            }

            if (leases.TryGetValue(address.Value, out MaterialLease retained))
            {
                return retained;
            }

            IMasonryAssetLease lease = preparedAssets!.Acquire(new PreparedAsset.Material(address));
            if (lease.Value is not Material material)
            {
                lease.Dispose();
                throw new MasonryWorldException(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared material '{address.Value}' is not a Unity Material."
                );
            }

            var created = new MaterialLease(address.Value, material, lease);
            leases.Add(address.Value, created);
            return created;
        }

        private void ReleaseUnused(MaterialLease lease)
        {
            if (lease.SlotCount != 0)
            {
                return;
            }

            leases.Remove(lease.Address);
            lease.Lease.Dispose();
        }

        private void OnDestroy() => Release();

        private static MasonryWorldException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private sealed class MaterialLease
        {
            public MaterialLease(string address, Material material, IMasonryAssetLease lease)
            {
                Address = address;
                Material = material;
                Lease = lease;
            }

            public string Address { get; }

            public Material Material { get; }

            public IMasonryAssetLease Lease { get; }

            public int SlotCount { get; set; }
        }
    }
}
