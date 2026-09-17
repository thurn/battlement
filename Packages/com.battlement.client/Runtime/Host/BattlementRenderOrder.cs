#nullable enable

using System;
using TMPro;
using UnityEngine;
using UnityEngine.Rendering;

namespace Battlement
{
    // One bounded owner per ordered object; disabling restores authored component state.
    internal sealed class BattlementRenderOrder : MonoBehaviour
    {
        private Renderer? target;
        private TextMeshPro? text;
        private SortingGroup? group;
        private int originalRendererOrder;
        private int originalGroupOrder;
        private bool originalGroupEnabled;
        private bool originalSortAtRoot;

        internal static void Apply(GameObject owner, RenderOrder? order)
        {
            BattlementRenderOrder state = owner.GetComponent<BattlementRenderOrder>();
            if (state == null && order is null)
                return;
            if (order is RenderOrder requested)
            {
                if (!Enum.IsDefined(typeof(RenderOrderKind), requested.Kind))
                    throw new ArgumentOutOfRangeException(nameof(order));
                if (
                    requested.Kind == RenderOrderKind.Layer
                    && owner.GetComponent<Renderer>() == null
                )
                    throw new InvalidOperationException("Layer ordering requires a renderer.");
            }
            if (state == null)
            {
                state = owner.AddComponent<BattlementRenderOrder>();
                state.Capture();
            }
            state.Restore();
            if (order is not RenderOrder value)
                return;
            if (value.Kind == RenderOrderKind.Group)
            {
                if (state.group == null)
                    state.group = owner.AddComponent<SortingGroup>();
                state.group.enabled = true;
                state.group.sortAtRoot = false;
                state.group.sortingOrder = value.Order;
            }
            else
                state.SetRendererOrder(value.Order);
        }

        private void Capture()
        {
            TryGetComponent(out target);
            TryGetComponent(out text);
            TryGetComponent(out group);
            if (target != null)
                originalRendererOrder = target.sortingOrder;
            if (group != null)
            {
                originalGroupOrder = group.sortingOrder;
                originalGroupEnabled = group.enabled;
                originalSortAtRoot = group.sortAtRoot;
            }
        }

        private void Restore()
        {
            SetRendererOrder(originalRendererOrder);
            if (group == null)
                return;
            group.sortingOrder = originalGroupOrder;
            group.sortAtRoot = originalSortAtRoot;
            group.enabled = originalGroupEnabled;
        }

        private void SetRendererOrder(int order)
        {
            if (text != null)
                text.sortingOrder = order;
            else if (target != null)
                target.sortingOrder = order;
        }
    }
}
