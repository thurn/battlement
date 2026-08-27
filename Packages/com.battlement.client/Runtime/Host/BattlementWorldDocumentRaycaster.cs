#nullable enable

using System.Collections.Generic;
using UnityEngine.EventSystems;
using UnityEngine.UIElements;

namespace Battlement
{
    internal sealed class BattlementWorldDocumentRaycaster : WorldDocumentRaycaster
    {
        public override void Raycast(
            PointerEventData eventData,
            List<RaycastResult> resultAppendList
        )
        {
            int firstResult = resultAppendList.Count;
            base.Raycast(eventData, resultAppendList);
            for (int index = firstResult; index < resultAppendList.Count; index++)
            {
                RaycastResult result = resultAppendList[index];
                if (result.gameObject == gameObject)
                    continue;
                result.sortingOrder = int.MaxValue;
                resultAppendList[index] = result;
            }
        }
    }
}
