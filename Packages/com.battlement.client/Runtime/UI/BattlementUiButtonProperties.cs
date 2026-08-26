#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiButtonProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, IconLease> leases = new();

        public BattlementUiButtonProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public IBattlementUiAssetLease? Stage(IconSource? source)
        {
            if (source is null)
                return null;
            if (assets is null)
                throw Failure(CoreErrorCode.AssetNotPrepared, "No UI asset lookup is configured.");
            IBattlementUiAssetLease lease = assets.Acquire(Prepared(source));
            if (HasExpectedType(source, lease.Value))
                return lease;
            lease.Dispose();
            throw Failure(
                CoreErrorCode.AssetTypeMismatch,
                $"Prepared button icon '{Address(source)}' has the wrong Unity type."
            );
        }

        public void Apply(
            UnityEngine.UIElements.Button target,
            ObjectId objectId,
            UiElement.Button value,
            IBattlementUiAssetLease? replacement
        )
        {
            BattlementUiTypographyProperties.Apply(target, value);
            if (value.Text is string text)
                target.text = text;
            if (value.Icon is null)
                return;
            target.iconImage = ToUnity(value.Icon, replacement!.Value);
            leases.Remove(objectId.Value, out IconLease previous);
            leases.Add(objectId.Value, new IconLease(value.Icon, replacement));
            previous?.Lease.Dispose();
        }

        public void Remove(Guid objectId)
        {
            if (leases.Remove(objectId, out IconLease retained))
                retained.Lease.Dispose();
        }

        public void Clear()
        {
            foreach (IconLease retained in leases.Values)
                retained.Lease.Dispose();
            leases.Clear();
        }

        private static Background ToUnity(IconSource source, object value) =>
            source switch
            {
                IconSource.Texture => Background.FromTexture2D((Texture2D)value),
                IconSource.Sprite => Background.FromSprite((Sprite)value),
                IconSource.VectorImage => Background.FromVectorImage((VectorImage)value),
                IconSource.RenderTexture => Background.FromRenderTexture((RenderTexture)value),
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown button icon kind."),
            };

        private static PreparedAsset Prepared(IconSource source) =>
            source switch
            {
                IconSource.Texture value => new PreparedAsset.Texture(value.Address),
                IconSource.Sprite value => new PreparedAsset.Sprite(value.Address),
                IconSource.VectorImage value => new PreparedAsset.VectorImage(value.Address),
                IconSource.RenderTexture value => new PreparedAsset.RenderTexture(value.Address),
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown button icon kind."),
            };

        private static bool HasExpectedType(IconSource source, object value) =>
            source switch
            {
                IconSource.Texture => value is Texture2D,
                IconSource.Sprite => value is Sprite,
                IconSource.VectorImage => value is VectorImage,
                IconSource.RenderTexture => value is RenderTexture,
                _ => false,
            };

        private static string Address(IconSource source) =>
            source switch
            {
                IconSource.Texture value => value.Address.Value,
                IconSource.Sprite value => value.Address.Value,
                IconSource.VectorImage value => value.Address.Value,
                IconSource.RenderTexture value => value.Address.Value,
                _ => throw Failure(CoreErrorCode.UnknownAsset, "Unknown button icon kind."),
            };

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);

        private sealed record IconLease(IconSource Source, IBattlementUiAssetLease Lease);
    }
}
