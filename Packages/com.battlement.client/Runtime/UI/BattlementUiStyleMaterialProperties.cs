#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Battlement.UI
{
    internal sealed class BattlementUiStyleMaterialProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, MaterialLease> leases = new();

        public BattlementUiStyleMaterialProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public IBattlementUiAssetLease? Stage(UiStyle? style)
        {
            UiStyleValue<MaterialAddress>? property = style?.UnityMaterial;
            if (property is null || property.Keyword is UiInlineKeyword.Initial)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            var prepared = new PreparedAsset.Material(property.Value);
            IBattlementUiAssetLease lease = assets.Acquire(prepared);
            if (lease.Value is Material)
                return lease;
            lease.Dispose();
            throw Failure(
                CoreErrorCode.AssetTypeMismatch,
                $"Prepared UI material '{property.Value.Value}' has the wrong Unity type."
            );
        }

        public void Commit(Guid objectId, UiStyle? style, IBattlementUiAssetLease? replacement)
        {
            UiStyleValue<MaterialAddress>? property = style?.UnityMaterial;
            if (property is null)
                return;
            leases.Remove(objectId, out MaterialLease previous);
            if (property.Keyword is null)
                leases.Add(objectId, new MaterialLease(property.Value, replacement!));
            previous?.Lease.Dispose();
        }

        public void Remove(Guid objectId)
        {
            if (leases.Remove(objectId, out MaterialLease retained))
                retained.Lease.Dispose();
        }

        public void Clear()
        {
            foreach (MaterialLease retained in leases.Values)
                retained.Lease.Dispose();
            leases.Clear();
        }

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);

        private sealed class MaterialLease
        {
            public MaterialLease(MaterialAddress address, IBattlementUiAssetLease lease)
            {
                Address = address;
                Lease = lease;
            }

            public MaterialAddress Address { get; }

            public IBattlementUiAssetLease Lease { get; }
        }
    }
}
