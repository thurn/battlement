#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementDocumentReconstruction
    {
        public static (UiDocument Description, UIDocument Document)[] Resolve(
            IReadOnlyList<UiDocument> descriptions,
            Func<ObjectId, GameObject?> resolveGameObject
        )
        {
            var result = new (UiDocument Description, UIDocument Document)[descriptions.Count];
            var roots = new HashSet<VisualElement>();
            for (int index = 0; index < descriptions.Count; index++)
            {
                UiDocument description = descriptions[index];
                GameObject? gameObject = resolveGameObject(description.DocumentId);
                if (gameObject == null)
                {
                    throw new InvalidOperationException(
                        $"UI document {description.DocumentId} has no owning GameObject."
                    );
                }
                if (!gameObject.TryGetComponent(out UIDocument document))
                {
                    throw new InvalidOperationException(
                        $"UI document {description.DocumentId} has no UIDocument component."
                    );
                }
                if (!roots.Add(document.rootVisualElement))
                {
                    throw new InvalidOperationException(
                        "An authoritative snapshot cannot assign one UIDocument to multiple roots."
                    );
                }
                result[index] = (description, document);
            }
            return result;
        }
    }
}
