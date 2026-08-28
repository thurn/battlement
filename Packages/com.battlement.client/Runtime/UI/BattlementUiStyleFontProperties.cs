#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.TextCore.Text;

namespace Battlement.UI
{
    internal sealed class BattlementUiStyleFontProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, FontLeases> leases = new();

        public BattlementUiStyleFontProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public FontLeases Stage(UiStyle? style) =>
            new(StageFontDefinition(style?.UnityFontDefinition ?? default));

        public void Commit(Guid objectId, UiStyle? style, FontLeases replacement)
        {
            Prop<UiStyleValue<UiFontAddress>> property = style?.UnityFontDefinition ?? default;
            if (property.IsUnset)
                return;
            leases.Remove(objectId, out FontLeases previous);
            leases.Add(objectId, replacement);
            previous?.Dispose();
        }

        public void Remove(Guid objectId)
        {
            if (leases.Remove(objectId, out FontLeases value))
                value.Dispose();
        }

        public void Clear()
        {
            foreach (FontLeases value in leases.Values)
                value.Dispose();
            leases.Clear();
        }

        private IBattlementUiAssetLease? StageFontDefinition(
            Prop<UiStyleValue<UiFontAddress>> property
        )
        {
            if (!property.IsSet || property.Value!.Keyword is UiInlineKeyword.Initial)
                return null;
            IBattlementUiAssetLease lease = Acquire(new PreparedAsset.UiFont(property.Value.Value));
            if (lease.Value is FontAsset)
                return lease;
            lease.Dispose();
            throw Failure("Prepared unity-font-definition has the wrong Unity type.");
        }

        private IBattlementUiAssetLease Acquire(PreparedAsset asset) =>
            assets?.Acquire(asset)
            ?? throw new BattlementUiException(
                CoreErrorCode.AssetNotPrepared,
                "No UI asset lookup is configured."
            );

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.AssetTypeMismatch, message);

        internal sealed class FontLeases : IDisposable
        {
            public FontLeases(IBattlementUiAssetLease? fontDefinition) =>
                FontDefinition = fontDefinition;

            public IBattlementUiAssetLease? FontDefinition { get; }

            public void Dispose() => FontDefinition?.Dispose();
        }
    }
}
