#nullable enable

using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.TextCore.Text;

namespace Battlement
{
    /// <summary>Resets glyph packing history in session-owned dynamic fonts.</summary>
    internal static class BattlementFontSession
    {
        // Stable atlas positions for common controls, independent of cached text request order.
        private static readonly string commonCharacters = new(
            Enumerable.Range(32, 95).Select(value => (char)value).ToArray()
        );

        public static void Reset(IEnumerable<object?> assets)
        {
            var visited = new HashSet<Object>();
            foreach (object? asset in assets)
            {
                Reset(asset, visited);
            }
        }

        internal static void Reset(object? asset, HashSet<Object> visited)
        {
            if (asset is not FontAsset && asset is not TMPro.TMP_FontAsset)
                return;
            if (asset is not Object value || value == null || !visited.Add(value))
            {
                return;
            }
            if (value is FontAsset font)
            {
                if (
                    font.atlasPopulationMode
                    is AtlasPopulationMode.Dynamic
                        or AtlasPopulationMode.DynamicOS
                )
                {
                    font.ClearFontAssetData();
                    font.TryAddCharacters(commonCharacters);
                }
                if (font.fallbackFontAssetTable is not null)
                {
                    foreach (FontAsset fallback in font.fallbackFontAssetTable)
                    {
                        Reset(fallback, visited);
                    }
                }
                foreach (FontWeightPair pair in font.fontWeightTable)
                {
                    Reset(pair.regularTypeface, visited);
                    Reset(pair.italicTypeface, visited);
                }
            }
            else if (value is TMPro.TMP_FontAsset textMeshFont)
            {
                if (textMeshFont.atlasPopulationMode != TMPro.AtlasPopulationMode.Static)
                {
                    textMeshFont.ClearFontAssetData();
                    textMeshFont.TryAddCharacters(commonCharacters);
                }
                if (textMeshFont.fallbackFontAssetTable is not null)
                {
                    foreach (TMPro.TMP_FontAsset fallback in textMeshFont.fallbackFontAssetTable)
                    {
                        Reset(fallback, visited);
                    }
                }
                foreach (TMPro.TMP_FontWeightPair pair in textMeshFont.fontWeightTable)
                {
                    Reset(pair.regularTypeface, visited);
                    Reset(pair.italicTypeface, visited);
                }
            }
        }
    }
}
