#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class BattlementUiPointerCrossings
    {
        private readonly Dictionary<int, PickedPath> currentPaths = new();
        private readonly Dictionary<int, Crossing> crossings = new();
        private readonly HashSet<int> cancelledPointers = new();
        private readonly Func<VisualElement?, Guid?> nearestId;

        public BattlementUiPointerCrossings(Func<VisualElement?, Guid?> nearestOwnedId) =>
            nearestId = nearestOwnedId;

        public void Clear()
        {
            currentPaths.Clear();
            crossings.Clear();
            cancelledPointers.Clear();
        }

        public void Cancel(int pointerId)
        {
            currentPaths.Remove(pointerId);
            crossings.Remove(pointerId);
            cancelledPointers.Add(pointerId);
        }

        public ObjectId? RelatedTarget(
            VisualElement root,
            EventBase eventBase,
            IPointerEvent eventValue,
            UiEventKind kind
        )
        {
            if (kind is not UiEventKind.PointerOver and not UiEventKind.PointerOut)
                throw new InvalidOperationException("Only pointer crossings have related targets.");

            if (cancelledPointers.Contains(eventValue.pointerId))
            {
                if (kind == UiEventKind.PointerOut)
                    return null;
                cancelledPointers.Remove(eventValue.pointerId);
                return null;
            }

            PickedPath targetPath = Path(eventBase.target as VisualElement);
            PickedPath nextPath = Pick(root, eventValue.position);
            if (
                crossings.TryGetValue(eventValue.pointerId, out Crossing crossing)
                && crossing.Completes(kind, targetPath.Target, nextPath.Target)
            )
            {
                crossings.Remove(eventValue.pointerId);
                currentPaths[eventValue.pointerId] = crossing.Next;
                return Owned(
                    kind == UiEventKind.PointerOver
                        ? crossing.Previous.Target
                        : crossing.Next.Target
                );
            }

            PickedPath previousPath = currentPaths.TryGetValue(
                eventValue.pointerId,
                out PickedPath remembered
            )
                ? remembered
                : PickedPath.Empty;
            if (kind == UiEventKind.PointerOut)
                previousPath = targetPath;
            Crossing started = new(previousPath, nextPath, kind);
            crossings[eventValue.pointerId] = started;
            currentPaths[eventValue.pointerId] = nextPath;
            return Owned(kind == UiEventKind.PointerOver ? previousPath.Target : nextPath.Target);
        }

        private PickedPath Pick(VisualElement root, UnityEngine.Vector3 position)
        {
            IPanel? panel = root.panel;
            return panel is null ? PickedPath.Empty : Path(panel.Pick(position));
        }

        private PickedPath Path(VisualElement? target)
        {
            var values = new List<Guid>();
            for (VisualElement? value = target; value is not null; value = value.parent)
            {
                if (nearestId(value) is not Guid objectId)
                    continue;
                if (values.Count == 0 || values[^1] != objectId)
                    values.Add(objectId);
            }
            return new PickedPath(values);
        }

        private static ObjectId? Owned(Guid? value) =>
            value is Guid objectId ? new ObjectId(objectId) : null;

        private sealed class PickedPath
        {
            public static PickedPath Empty { get; } = new(Array.Empty<Guid>());

            public PickedPath(IReadOnlyList<Guid> values) => Values = values;

            public IReadOnlyList<Guid> Values { get; }

            public Guid? Target => Values.Count == 0 ? null : Values[0];
        }

        private sealed class Crossing
        {
            public Crossing(PickedPath previous, PickedPath next, UiEventKind firstKind)
            {
                Previous = previous;
                Next = next;
                FirstKind = firstKind;
            }

            public PickedPath Previous { get; }

            public PickedPath Next { get; }

            public UiEventKind FirstKind { get; }

            public bool Completes(UiEventKind kind, Guid? target, Guid? picked) =>
                kind != FirstKind
                && target == (kind == UiEventKind.PointerOver ? Next.Target : Previous.Target)
                && picked == Next.Target;
        }
    }
}
