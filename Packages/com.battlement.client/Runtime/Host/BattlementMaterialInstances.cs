#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace Battlement
{
    // Renderer-local property blocks preserve the shared material and bounded asset leases.
    [DisallowMultipleComponent]
    internal sealed class BattlementMaterialInstances : MonoBehaviour, IBattlementOwnedResource
    {
        private Renderer? target;
        private BattlementImage? image;
        private Dictionary<int, Entry> entries = new();
        private readonly Dictionary<int, Original> originals = new();

        internal static void Apply(
            GameObject owner,
            BattlementPreparedAssets assets,
            IReadOnlyList<MaterialInstance>? values
        )
        {
            assets.ValidateMaterialInstances(values);
            owner.TryGetComponent(out BattlementMaterialInstances state);
            if (state == null && (values == null || values.Count == 0))
                return;
            if (!owner.TryGetComponent(out Renderer renderer))
                throw new BattlementWorldException(
                    CoreErrorCode.ComponentMissing,
                    "Material overrides require a renderer."
                );
            if (state == null)
            {
                state = owner.AddComponent<BattlementMaterialInstances>();
                state.target = renderer;
                owner.TryGetComponent(out state.image);
            }
            state.Replace(assets, values ?? Array.Empty<MaterialInstance>());
        }

        private void Replace(
            BattlementPreparedAssets assets,
            IReadOnlyList<MaterialInstance> values
        )
        {
            Material[] materials = target!.sharedMaterials;
            var next = new Dictionary<int, Entry>();
            try
            {
                foreach (MaterialInstance value in values)
                {
                    if (value.Slot >= materials.Length || next.ContainsKey((int)value.Slot))
                        throw BattlementMaterialParameters.Invalid(
                            "Invalid or duplicate material slot."
                        );
                    IBattlementAssetLease lease = assets.Acquire(
                        new PreparedAsset.Material(value.Address)
                    );
                    try
                    {
                        if (lease.Value is not Material material)
                            throw BattlementMaterialParameters.Invalid(
                                "Prepared material is not a Unity Material."
                            );
                        var names = new HashSet<string>(StringComparer.Ordinal);
                        foreach (MaterialParameterValue p in value.Parameters)
                        {
                            if (string.IsNullOrEmpty(p.Name) || !names.Add(p.Name))
                                throw BattlementMaterialParameters.Invalid(
                                    "Invalid or duplicate material parameter."
                                );
                            BattlementMaterialParameters.Validate(material, p.Name, p.Kind);
                            foreach (double number in new[] { p.X, p.Y, p.Z, p.W })
                                if (double.IsNaN(number) || Math.Abs(number) > float.MaxValue)
                                    throw BattlementMaterialParameters.Invalid(
                                        "Material values must be finite shader numbers."
                                    );
                        }
                        next.Add(
                            (int)value.Slot,
                            new Entry(
                                material,
                                value.Parameters,
                                lease,
                                new MaterialPropertyBlock()
                            )
                        );
                    }
                    catch
                    {
                        lease.Dispose();
                        throw;
                    }
                }
            }
            catch
            {
                foreach (Entry entry in next.Values)
                    entry.Lease.Dispose();
                throw;
            }

            foreach (int slot in entries.Keys)
            {
                Original original = originals[slot];
                materials[slot] = original.Material;
                target.SetPropertyBlock(original.Block.isEmpty ? null : original.Block, slot);
            }
            foreach ((int slot, Entry entry) in next)
            {
                if (!originals.ContainsKey(slot))
                {
                    var block = new MaterialPropertyBlock();
                    target.GetPropertyBlock(block, slot);
                    originals.Add(slot, new Original(materials[slot], block));
                }
                materials[slot] = entry.Material;
            }
            target.sharedMaterials = materials;
            foreach (Entry entry in entries.Values)
                entry.Lease.Dispose();
            entries = next;
            var removed = new List<int>();
            foreach (int slot in originals.Keys)
                if (!entries.ContainsKey(slot))
                    removed.Add(slot);
            foreach (int slot in removed)
                originals.Remove(slot);
            Refresh();
        }

        internal void Refresh()
        {
            foreach ((int slot, Entry entry) in entries)
            {
                MaterialPropertyBlock block = entry.Block;
                block.Clear();
                Original original = originals[slot];
                target!.SetPropertyBlock(original.Block.isEmpty ? null : original.Block, slot);
                target.GetPropertyBlock(block, slot);
                if (block.isEmpty)
                    target.GetPropertyBlock(block);
                if (image != null)
                    image.WriteMaterialProperties(entry.Material, block);
                foreach (MaterialParameterValue p in entry.Parameters)
                {
                    switch (p.Kind)
                    {
                        case MaterialParameterKind.Float:
                            block.SetFloat(p.Name, (float)p.X);
                            break;
                        case MaterialParameterKind.Color:
                            block.SetColor(
                                p.Name,
                                new UnityEngine.Color(
                                    (float)p.X,
                                    (float)p.Y,
                                    (float)p.Z,
                                    (float)p.W
                                )
                            );
                            break;
                        case MaterialParameterKind.Vector:
                            block.SetVector(
                                p.Name,
                                new Vector4((float)p.X, (float)p.Y, (float)p.Z, (float)p.W)
                            );
                            break;
                        default:
                            throw new InvalidOperationException("Unknown material parameter kind.");
                    }
                }
                foreach ((string name, float value) in entry.MotionScalars)
                    block.SetFloat(name, value);
                target.SetPropertyBlock(block, slot);
            }
        }

        internal bool SupportsMotionScalar(uint slot, string parameter) =>
            entries.TryGetValue((int)slot, out Entry entry)
            && entry.Parameters.Any(value =>
                value.Name == parameter && value.Kind == MaterialParameterKind.Float
            );

        internal float ReadMotionScalar(uint slot, string parameter)
        {
            Entry entry = RequireMotionScalar(slot, parameter);
            if (entry.MotionScalars.TryGetValue(parameter, out float presented))
                return presented;
            return (float)entry.Parameters.First(value => value.Name == parameter).X;
        }

        internal void WriteMotionScalar(uint slot, string parameter, float value)
        {
            Entry entry = RequireMotionScalar(slot, parameter);
            entry.MotionScalars[parameter] = value;
            entry.Block.SetFloat(parameter, value);
            target!.SetPropertyBlock(entry.Block, (int)slot);
        }

        internal void ClearMotionScalar(uint slot, string parameter)
        {
            if (
                entries.TryGetValue((int)slot, out Entry entry)
                && entry.MotionScalars.Remove(parameter)
            )
                Refresh();
        }

        private Entry RequireMotionScalar(uint slot, string parameter)
        {
            if (!SupportsMotionScalar(slot, parameter))
                throw BattlementMaterialParameters.Invalid(
                    $"Material Motion float parameter '{parameter}' is not prepared in slot {slot}."
                );
            return entries[(int)slot];
        }

        internal void Release()
        {
            foreach (Entry entry in entries.Values)
                entry.Lease.Dispose();
            entries.Clear();
            originals.Clear();
        }

        void IBattlementOwnedResource.Release() => Release();

        private void OnDestroy() => Release();

        private sealed record Entry(
            Material Material,
            IReadOnlyList<MaterialParameterValue> Parameters,
            IBattlementAssetLease Lease,
            MaterialPropertyBlock Block
        )
        {
            internal Dictionary<string, float> MotionScalars { get; } = new();
        }

        private sealed record Original(Material Material, MaterialPropertyBlock Block);
    }
}
