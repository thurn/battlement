#nullable enable

using System.Collections.Generic;
using UnityEngine;
using UnityEngine.TextCore.Text;

namespace Battlement
{
    /// <summary>Resets glyph packing history in session-owned dynamic fonts.</summary>
    internal static class BattlementFontSession
    {
        public static void Reset(IEnumerable<object?> assets)
        {
            var visited = new HashSet<Object>();
            foreach (object? asset in assets)
            {
                Reset(asset, visited);
            }
        }

        private static void Reset(object? asset, HashSet<Object> visited)
        {
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
