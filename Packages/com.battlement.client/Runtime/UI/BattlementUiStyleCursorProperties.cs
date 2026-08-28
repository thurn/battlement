#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;

namespace Battlement.UI
{
    internal sealed class BattlementUiStyleCursorProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, IBattlementUiAssetLease> leases = new();

        public BattlementUiStyleCursorProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public IBattlementUiAssetLease? Stage(UiStyle? style)
        {
            Prop<UiStyleValue<UiCursor>> property = style?.Cursor ?? default;
            if (!property.IsSet || property.Value!.Keyword is UiInlineKeyword.Initial)
                return null;
            if (property.Value.Value is UiCursor.Default)
                return null;
            if (property.Value.Value is not UiCursor.Texture cursor)
                throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI cursor kind.");
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            IBattlementUiAssetLease lease = assets.Acquire(
                new PreparedAsset.Texture(cursor.Address)
            );
            if (lease.Value is not Texture2D texture)
            {
                lease.Dispose();
                throw Failure(
                    CoreErrorCode.AssetTypeMismatch,
                    $"Prepared UI cursor '{cursor.Address.Value}' is not a Texture2D."
                );
            }
            bool inside = cursor.Hotspot.X < texture.width && cursor.Hotspot.Y < texture.height;
            if (!texture.isReadable || !inside)
            {
                lease.Dispose();
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "A UI cursor texture must be readable and contain its hotspot."
                );
            }
            return lease;
        }

        public void Commit(Guid objectId, UiStyle? style, IBattlementUiAssetLease? replacement)
        {
            Prop<UiStyleValue<UiCursor>> property = style?.Cursor ?? default;
            if (property.IsUnset)
                return;
            leases.Remove(objectId, out IBattlementUiAssetLease previous);
            if (
                property.IsSet
                && property.Value!.Keyword is null
                && property.Value.Value is UiCursor.Texture
            )
                leases.Add(objectId, replacement!);
            previous?.Dispose();
        }

        public void Remove(Guid objectId)
        {
            if (leases.Remove(objectId, out IBattlementUiAssetLease retained))
                retained.Dispose();
        }

        public void Clear()
        {
            foreach (IBattlementUiAssetLease retained in leases.Values)
                retained.Dispose();
            leases.Clear();
        }

        public static UnityEngine.UIElements.Cursor ToUnity(UiCursor value, object? texture) =>
            value switch
            {
                UiCursor.Default => default,
                UiCursor.Texture cursor => new UnityEngine.UIElements.Cursor
                {
                    texture = (Texture2D)texture!,
                    hotspot = new Vector2(cursor.Hotspot.X, cursor.Hotspot.Y),
                },
                _ => throw Failure(CoreErrorCode.InvalidProperty, "Unknown UI cursor kind."),
            };

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
