#nullable enable
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiFocusTargets
    {
        internal static IEnumerable<(VisualElement Element, UIDocument Document)> Collect(
            BattlementUiHierarchy hierarchy,
            BattlementFocusCoordinator focus
        )
        {
            foreach (
                var entry in hierarchy
                    .Entries.OrderBy(e => e.Element.tabIndex > 0 ? 0 : 1)
                    .ThenBy(e => e.Element.tabIndex)
                    .ThenBy(e => hierarchy.SourceOrdinal(e.Element))
            )
            {
                VisualElement element = entry.Element;
                if (element.panel == null || !element.focusable || !element.enabledInHierarchy)
                    continue;
                if (element.tabIndex < 0 || focus.IsEffectivelyInert(entry.Id))
                    continue;
                if (
                    element.resolvedStyle.display == DisplayStyle.None
                    || element.resolvedStyle.visibility != Visibility.Visible
                )
                    continue;
                if (
                    !hierarchy.TryGetGeometryTarget(
                        new ObjectId(entry.Id),
                        out _,
                        out ObjectId panel,
                        out UIDocument document
                    )
                )
                    continue;
                if (element.worldBound.width <= 0 || element.worldBound.height <= 0)
                    continue;
                yield return (element, document);
            }
        }
    }
}
