#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.TextCore.Text;

namespace Battlement.UI
{
    internal sealed class BattlementUiStyleFontProperties
    {
        private readonly IBattlementUiAssetLookup? assets;
        private readonly Dictionary<Guid, FontLeases> leases = new();

        public BattlementUiStyleFontProperties(IBattlementUiAssetLookup? assets) =>
            this.assets = assets;

        public FontLeases Stage(UiStyle? style)
        {
            IBattlementUiAssetLease? unityFont = StageUnityFont(style?.UnityFont);
            try
            {
                return new FontLeases(unityFont, StageFontDefinition(style?.UnityFontDefinition));
            }
            catch
            {
                unityFont?.Dispose();
                throw;
            }
        }

        public void Commit(Guid objectId, UiStyle? style, FontLeases replacement)
        {
            if (style?.UnityFont is null && style?.UnityFontDefinition is null)
                return;
            leases.Remove(objectId, out FontLeases previous);
            leases.Add(
                objectId,
                new FontLeases(
                    style?.UnityFont is null ? previous?.UnityFont : replacement.UnityFont,
                    style?.UnityFontDefinition is null
                        ? previous?.FontDefinition
                        : replacement.FontDefinition
                )
            );
            if (style?.UnityFont is not null)
                previous?.UnityFont?.Dispose();
            if (style?.UnityFontDefinition is not null)
                previous?.FontDefinition?.Dispose();
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

        private IBattlementUiAssetLease? StageUnityFont(UiStyleValue<UnityFontAddress>? property)
        {
            if (property is null || property.Keyword is UiInlineKeyword.Initial)
                return null;
            IBattlementUiAssetLease lease = Acquire(new PreparedAsset.UnityFont(property.Value));
            if (lease.Value is Font)
                return lease;
            lease.Dispose();
            throw Failure("Prepared unity-font has the wrong Unity type.");
        }

        private IBattlementUiAssetLease? StageFontDefinition(UiStyleValue<UiFontAddress>? property)
        {
            if (property is null || property.Keyword is UiInlineKeyword.Initial)
                return null;
            IBattlementUiAssetLease lease = Acquire(new PreparedAsset.UiFont(property.Value));
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
            public FontLeases(
                IBattlementUiAssetLease? unityFont,
                IBattlementUiAssetLease? fontDefinition
            )
            {
                UnityFont = unityFont;
                FontDefinition = fontDefinition;
            }

            public IBattlementUiAssetLease? UnityFont { get; }
            public IBattlementUiAssetLease? FontDefinition { get; }

            public void Dispose()
            {
                UnityFont?.Dispose();
                FontDefinition?.Dispose();
            }
        }
    }
}
