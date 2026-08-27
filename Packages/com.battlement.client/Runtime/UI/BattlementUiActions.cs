#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiActions
    {
        private readonly Dictionary<Guid, HashSet<int>> captures = new();
        private readonly Func<ObjectId, VisualElement> require;
        private readonly Func<Guid, Guid, bool> isDescendant;
        private readonly BattlementUiScrollControls scrollControls;

        public BattlementUiActions(
            Func<ObjectId, VisualElement> requireElement,
            Func<Guid, Guid, bool> descendantCheck,
            BattlementUiScrollControls scrollControlManager
        ) =>
            (require, isDescendant, scrollControls) = (
                requireElement,
                descendantCheck,
                scrollControlManager
            );

        public void Perform(CommandBody.VisualElement.PerformAction command)
        {
            VisualElement target = require(command.ObjectId);
            switch (command.Action)
            {
                case VisualElementAction.Focus:
                    Focus(target);
                    break;
                case VisualElementAction.Blur:
                    Blur(target);
                    break;
                case VisualElementAction.CapturePointer capture:
                    Capture(target, command.ObjectId, capture.PointerId);
                    break;
                case VisualElementAction.ReleasePointer release:
                    Release(target, command.ObjectId, release.PointerId);
                    break;
                case VisualElementAction.ScrollTo scrollTo:
                    ScrollTo(target, command.ObjectId, scrollTo.DescendantId);
                    break;
                case VisualElementAction.SelectText selection:
                    SelectText(target, selection.CursorIndex, selection.SelectionIndex);
                    break;
                default:
                    throw Failure("The UI action is unsupported by this executor.");
            }
        }

        public void Remove(ObjectId objectId, VisualElement target)
        {
            ReleaseTracked(objectId.Value, target);
            if (target.panel?.focusController.focusedElement == target)
                target.Blur();
        }

        public void CancelAll(IReadOnlyDictionary<Guid, VisualElement> elements)
        {
            foreach ((Guid id, VisualElement target) in elements)
                ReleaseTracked(id, target);
            captures.Clear();

            var panels = new HashSet<IPanel>();
            foreach (VisualElement target in elements.Values)
            {
                if (target.panel is IPanel panel)
                    panels.Add(panel);
            }
            foreach (IPanel panel in panels)
            {
                if (panel.focusController.focusedElement is Focusable focused)
                    focused.Blur();
            }
        }

        private static void Focus(VisualElement target)
        {
            if (target.panel is null || !target.enabledInHierarchy || !target.focusable)
                throw Failure("Focus requires an attached, enabled, focusable element.");
            target.Focus();
        }

        private static void Blur(VisualElement target)
        {
            if (target.panel is null || target.panel.focusController.focusedElement != target)
                throw Failure("Blur requires the target to own focus in its panel.");
            target.Blur();
        }

        private void Capture(VisualElement target, ObjectId objectId, int pointerId)
        {
            if (target.panel is null || !target.enabledInHierarchy)
                throw Failure("CapturePointer requires an attached, enabled element.");
            target.CapturePointer(pointerId);
            if (!target.HasPointerCapture(pointerId))
                throw Failure("The pointer could not be captured by the target.");
            if (!captures.TryGetValue(objectId.Value, out HashSet<int> pointerIds))
            {
                pointerIds = new HashSet<int>();
                captures.Add(objectId.Value, pointerIds);
            }
            pointerIds.Add(pointerId);
        }

        private void Release(VisualElement target, ObjectId objectId, int pointerId)
        {
            if (!target.HasPointerCapture(pointerId))
                throw Failure("ReleasePointer requires the target to own the pointer capture.");
            target.ReleasePointer(pointerId);
            if (captures.TryGetValue(objectId.Value, out HashSet<int> pointerIds))
            {
                pointerIds.Remove(pointerId);
                if (pointerIds.Count == 0)
                    captures.Remove(objectId.Value);
            }
        }

        private void ScrollTo(VisualElement target, ObjectId objectId, ObjectId descendantId)
        {
            if (target is not ScrollView scroll)
                throw ComponentFailure("ScrollTo requires a ScrollView.");
            VisualElement descendant = require(descendantId);
            if (descendantId == objectId || !isDescendant(descendantId.Value, objectId.Value))
                throw HierarchyFailure("ScrollTo requires a logical descendant of its target.");
            scrollControls.ScrollTo(objectId, scroll, descendant);
        }

        private static void SelectText(VisualElement target, uint cursorIndex, uint selectionIndex)
        {
            string text;
            ITextSelection selection;
            if (target is TextField field)
            {
                TextElement input = field
                    .Q<VisualElement>(TextField.textInputUssName)
                    .Q<TextElement>();
                text = input.text ?? string.Empty;
                selection = field.textSelection;
            }
            else if (target is TextElement textElement)
            {
                selection = textElement;
                if (!selection.isSelectable)
                    throw Failure("SelectText requires selectable text.");
                text = textElement.text ?? string.Empty;
            }
            else
            {
                throw ComponentFailure("SelectText requires selectable text or a text input.");
            }
            if (cursorIndex > text.Length || selectionIndex > text.Length)
                throw Failure("SelectText indices must be within the current UTF-16 text.");
            int cursor = checked((int)cursorIndex);
            int selected = checked((int)selectionIndex);
            selection.SelectRange(cursor, selected);
            if (selection.cursorIndex != cursor)
                selection.cursorIndex = cursor;
            if (selection.selectIndex != selected)
                selection.selectIndex = selected;
        }

        private void ReleaseTracked(Guid objectId, VisualElement target)
        {
            if (!captures.Remove(objectId, out HashSet<int> pointerIds))
                return;
            foreach (int pointerId in pointerIds)
            {
                if (target.HasPointerCapture(pointerId))
                    target.ReleasePointer(pointerId);
            }
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private static BattlementUiException ComponentFailure(string message) =>
            new(CoreErrorCode.ComponentMissing, message);

        private static BattlementUiException HierarchyFailure(string message) =>
            new(CoreErrorCode.InvalidHierarchy, message);
    }
}
