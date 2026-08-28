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
            Prop<UiStyleValue<MaterialAddress>> property = style?.UnityMaterial ?? default;
            if (!property.IsSet || property.Value!.Keyword is UiInlineKeyword.Initial)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            var prepared = new PreparedAsset.Material(property.Value.Value);
            IBattlementUiAssetLease lease = assets.Acquire(prepared);
            if (lease.Value is Material)
                return lease;
            lease.Dispose();
            throw Failure(
                CoreErrorCode.AssetTypeMismatch,
                $"Prepared UI material '{property.Value.Value.Value}' has the wrong Unity type."
            );
        }

        public void Commit(Guid objectId, UiStyle? style, IBattlementUiAssetLease? replacement)
        {
            Prop<UiStyleValue<MaterialAddress>> property = style?.UnityMaterial ?? default;
            if (property.IsUnset)
                return;
            leases.Remove(objectId, out MaterialLease previous);
            if (property.IsSet && property.Value!.Keyword is null)
                leases.Add(objectId, new MaterialLease(property.Value.Value, replacement!));
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
