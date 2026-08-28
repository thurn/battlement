#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiStyleBackgroundProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, BackgroundLease> leases = new();

        public BattlementUiStyleBackgroundProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public IBattlementUiAssetLease? Stage(UiStyle? style)
        {
            var property = style?.BackgroundImage ?? default;
            if (!property.IsSet || property.Value!.Keyword is UiInlineKeyword.Initial)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            IBattlementUiAssetLease lease = assets.Acquire(Prepared(property.Value.Value));
            if (HasExpectedType(property.Value.Value, lease.Value))
                return lease;
            lease.Dispose();
            throw Failure(
                CoreErrorCode.AssetTypeMismatch,
                $"Prepared background '{Address(property.Value.Value)}' has the wrong Unity type."
            );
        }

        public void Commit(Guid objectId, UiStyle? style, IBattlementUiAssetLease? replacement)
        {
            Prop<UiStyleValue<BackgroundSource>> property = style?.BackgroundImage ?? default;
            if (property.IsUnset)
                return;
            leases.Remove(objectId, out BackgroundLease previous);
            if (property.IsSet && property.Value!.Keyword is null)
                leases.Add(objectId, new BackgroundLease(property.Value.Value, replacement!));
            previous?.Lease.Dispose();
        }

        public void Remove(Guid objectId)
        {
            if (leases.Remove(objectId, out BackgroundLease retained))
                retained.Lease.Dispose();
        }

        public void Clear()
        {
            foreach (BackgroundLease retained in leases.Values)
                retained.Lease.Dispose();
            leases.Clear();
        }

        public static Background ToUnity(BackgroundSource source, object value) =>
            source switch
            {
                BackgroundSource.Texture => Background.FromTexture2D((Texture2D)value),
                BackgroundSource.Sprite => Background.FromSprite((Sprite)value),
                BackgroundSource.VectorImage => Background.FromVectorImage((VectorImage)value),
                BackgroundSource.RenderTexture => Background.FromRenderTexture(
                    (RenderTexture)value
                ),
                _ => throw Failure(
                    CoreErrorCode.UnknownAsset,
                    "Unknown UI background source kind."
                ),
            };

        private static PreparedAsset Prepared(BackgroundSource source) =>
            source switch
            {
                BackgroundSource.Texture value => new PreparedAsset.Texture(value.Address),
                BackgroundSource.Sprite value => new PreparedAsset.Sprite(value.Address),
                BackgroundSource.VectorImage value => new PreparedAsset.VectorImage(value.Address),
                BackgroundSource.RenderTexture value => new PreparedAsset.RenderTexture(
                    value.Address
                ),
                _ => throw Failure(
                    CoreErrorCode.UnknownAsset,
                    "Unknown UI background source kind."
                ),
            };

        private static bool HasExpectedType(BackgroundSource source, object value) =>
            source switch
            {
                BackgroundSource.Texture => value is Texture2D,
                BackgroundSource.Sprite => value is Sprite,
                BackgroundSource.VectorImage => value is VectorImage,
                BackgroundSource.RenderTexture => value is RenderTexture,
                _ => false,
            };

        private static string Address(BackgroundSource source) =>
            source switch
            {
                BackgroundSource.Texture value => value.Address.Value,
                BackgroundSource.Sprite value => value.Address.Value,
                BackgroundSource.VectorImage value => value.Address.Value,
                BackgroundSource.RenderTexture value => value.Address.Value,
                _ => throw Failure(
                    CoreErrorCode.UnknownAsset,
                    "Unknown UI background source kind."
                ),
            };

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);

        private sealed class BackgroundLease
        {
            public BackgroundLease(BackgroundSource source, IBattlementUiAssetLease lease)
            {
                Source = source;
                Lease = lease;
            }

            public BackgroundSource Source { get; }

            public IBattlementUiAssetLease Lease { get; }
        }
    }
}
