#nullable enable

using System.Collections.Generic;
using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UIElements;

namespace Battlement
{
    internal sealed class BattlementPhysicsRaycaster : PhysicsRaycaster
    {
        private readonly List<RaycastResult> results = new();

        public override void Raycast(
            PointerEventData eventData,
            List<RaycastResult> resultAppendList
        )
        {
            results.Clear();
            base.Raycast(eventData, results);
            foreach (RaycastResult result in results)
            {
                if (!BattlementWorldDocumentCollider.IsGenerated(result.gameObject))
                    resultAppendList.Add(result);
            }
        }
    }

    internal static class BattlementWorldDocumentCollider
    {
        public static bool IsGenerated(GameObject gameObject) =>
            gameObject.TryGetComponent(out BattlementIdentity _)
            && gameObject.TryGetComponent(out UIDocument document)
            && document.panelSettings != null
            && document.panelSettings.renderMode
                == UnityEngine.UIElements.PanelRenderMode.WorldSpace;
    }
}
