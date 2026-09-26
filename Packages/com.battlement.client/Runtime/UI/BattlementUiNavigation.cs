#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiNavigation
    {
        private readonly Func<IEnumerable<UIDocument>> documents;
        private readonly System.Action showFocus;
        private readonly Func<IEnumerable<(VisualElement Element, UIDocument Document)>> targets;

        internal BattlementUiNavigation(
            Func<IEnumerable<UIDocument>> documents,
            System.Action showFocus,
            Func<IEnumerable<(VisualElement Element, UIDocument Document)>>? targets = null
        ) =>
            (this.documents, this.showFocus, this.targets) = (
                documents,
                showFocus,
                targets ?? (() => Array.Empty<(VisualElement, UIDocument)>())
            );

        internal IEnumerable<(VisualElement Element, UIDocument Document)> Targets => targets();

        internal VisualElement? Focused =>
            documents()
                .Where(d => d != null && d.isActiveAndEnabled)
                .Select(d =>
                    d.rootVisualElement.panel?.focusController.focusedElement as VisualElement
                )
                .FirstOrDefault(e => e != null);

        internal void Blur() => Focused?.Blur();

        internal void ShowFocus() => showFocus();

        internal void Navigate(UiNavigationDirection direction)
        {
            VisualElement? target = Focused;
            if (target == null)
                return;
            UnityEngine.Vector2 delta = direction switch
            {
                UiNavigationDirection.Left => UnityEngine.Vector2.left,
                UiNavigationDirection.Right => UnityEngine.Vector2.right,
                UiNavigationDirection.Up => UnityEngine.Vector2.up,
                UiNavigationDirection.Down => UnityEngine.Vector2.down,
                _ => UnityEngine.Vector2.zero,
            };
            if (direction is UiNavigationDirection.Next or UiNavigationDirection.Previous)
            {
                using NavigationMoveEvent move = NavigationMoveEvent.GetPooled(
                    direction == UiNavigationDirection.Previous
                        ? NavigationMoveEvent.Direction.Previous
                        : NavigationMoveEvent.Direction.Next
                );
                move.target = target;
                target.SendEvent(move);
            }
            else
            {
                using NavigationMoveEvent move = NavigationMoveEvent.GetPooled(delta);
                move.target = target;
                target.SendEvent(move);
            }
            showFocus();
        }

        internal void Activate()
        {
            VisualElement? target = Focused;
            if (target == null)
                return;
            using NavigationSubmitEvent value = NavigationSubmitEvent.GetPooled();
            value.target = target;
            target.SendEvent(value);
            showFocus();
        }

        internal void Cancel()
        {
            VisualElement? target = Focused;
            if (target == null)
                return;
            using NavigationCancelEvent value = NavigationCancelEvent.GetPooled();
            value.target = target;
            target.SendEvent(value);
            showFocus();
        }
    }
}
